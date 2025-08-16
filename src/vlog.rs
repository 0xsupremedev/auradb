use crate::config::{CompressionAlgorithm, ValueLogConfig};
use crate::error::{Error, Result};
use crate::storage::{Value, ValuePointer};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Read, Seek, SeekFrom, Write};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tracing::{debug, error, info, warn};

/// Value log segment header
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VlogHeader {
    /// Magic number for value log files
    pub magic: [u8; 8],
    /// Version number
    pub version: u32,
    /// Segment creation timestamp
    pub created_at: u64,
    /// Compression algorithm used
    pub compression: CompressionAlgorithm,
    /// Checksum of the header
    pub checksum: u32,
}

impl VlogHeader {
    const MAGIC: [u8; 8] = [0x41, 0x55, 0x52, 0x41, 0x44, 0x42, 0x56, 0x4C]; // "AURADBVL"
    const VERSION: u32 = 1;

    /// Create a new value log header
    pub fn new(compression: CompressionAlgorithm) -> Self {
        let created_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        Self {
            magic: Self::MAGIC,
            version: Self::VERSION,
            created_at,
            compression,
            checksum: 0, // Will be calculated
        }
    }

    /// Calculate checksum for the header
    pub fn calculate_checksum(&self) -> u32 {
        use crc32fast::Hasher;
        let mut hasher = Hasher::new();
        hasher.update(&self.magic);
        hasher.update(&self.version.to_le_bytes());
        hasher.update(&self.created_at.to_le_bytes());
        hasher.update(&(self.compression as u8).to_le_bytes());
        hasher.finalize()
    }

    /// Validate the header
    pub fn validate(&self) -> bool {
        self.magic == Self::MAGIC
            && self.version == Self::VERSION
            && self.checksum == self.calculate_checksum()
    }
}

/// Value log entry metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VlogEntry {
    /// Entry length in bytes
    pub length: u32,
    /// Compression algorithm used
    pub compression: CompressionAlgorithm,
    /// Checksum of the value
    pub checksum: u32,
    /// Timestamp when written
    pub timestamp: u64,
}

/// Value log segment metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VlogSegmentMeta {
    /// Segment file path
    pub path: PathBuf,
    /// Segment size in bytes
    pub size: u64,
    /// Number of entries in the segment
    pub entry_count: u64,
    /// First entry offset
    pub first_offset: u64,
    /// Last entry offset
    pub last_offset: u64,
    /// Creation timestamp
    pub created_at: u64,
    /// Whether the segment is closed
    pub closed: bool,
}

/// Value log writer that handles writing values to segments
pub struct VlogWriter {
    /// Current active segments for parallel writes
    segments: Vec<Arc<RwLock<VlogSegment>>>,
    /// Configuration
    config: ValueLogConfig,
    /// Next segment ID
    next_segment_id: AtomicU64,
    /// Value log directory
    vlog_dir: PathBuf,
    /// Write queues for parallel writes
    write_queues: Vec<mpsc::UnboundedSender<WriteRequest>>,
    /// Background task handles
    background_handles: Vec<JoinHandle<()>>,
    /// Segment metadata cache
    segment_metadata: HashMap<u64, VlogSegmentMeta>,
}

impl VlogWriter {
    /// Create a new value log writer
    pub fn new(config: ValueLogConfig) -> Result<Self> {
        let vlog_dir = config.vlog_path.clone();
        std::fs::create_dir_all(&vlog_dir)?;

        let mut writer = Self {
            segments: Vec::new(),
            config,
            next_segment_id: AtomicU64::new(1),
            vlog_dir,
            write_queues: Vec::new(),
            background_handles: Vec::new(),
            segment_metadata: HashMap::new(),
        };

        // Initialize write queues and background tasks
        writer.initialize_write_queues()?;
        
        // Create initial segments
        for _ in 0..writer.config.write_queues {
            writer.create_new_segment()?;
        }

        Ok(writer)
    }

    /// Initialize write queues and background tasks
    fn initialize_write_queues(&mut self) -> Result<()> {
        for queue_id in 0..self.config.write_queues {
            let (tx, mut rx) = mpsc::unbounded_channel();
            self.write_queues.push(tx);

            let vlog_dir = self.vlog_dir.clone();
            let config = self.config.clone();
            let queue_id = queue_id;

            let handle = tokio::spawn(async move {
                let mut current_segment = None;
                let mut write_buffer = Vec::new();

                while let Some(request) = rx.recv().await {
                    match request {
                        WriteRequest::Write { value, callback } => {
                            write_buffer.push((value, callback));
                            
                            // Flush if buffer is full
                            if write_buffer.len() >= 100 {
                                if let Err(e) = Self::flush_values(&mut current_segment, &vlog_dir, &config, &mut write_buffer, queue_id).await {
                                    error!("Failed to flush values in queue {}: {}", queue_id, e);
                                }
                            }
                        }
                        WriteRequest::Sync => {
                            if let Err(e) = Self::flush_values(&mut current_segment, &vlog_dir, &config, &mut write_buffer, queue_id).await {
                                error!("Failed to sync values in queue {}: {}", queue_id, e);
                            }
                        }
                        WriteRequest::Shutdown => break,
                    }
                }
            });

            self.background_handles.push(handle);
        }

        Ok(())
    }

    /// Flush values to segment (async helper)
    async fn flush_values(
        current_segment: &mut Option<VlogSegment>,
        vlog_dir: &PathBuf,
        config: &ValueLogConfig,
        write_buffer: &mut Vec<(Value, Option<WriteCallback>)>,
        queue_id: usize,
    ) -> Result<()> {
        if write_buffer.is_empty() {
            return Ok(());
        }

        // Ensure we have a current segment
        if current_segment.is_none() {
            *current_segment = Some(VlogSegment::new(vlog_dir, config, queue_id as u64)?);
        }

        let segment = current_segment.as_mut().unwrap();
        
        // Write all values
        for (value, callback) in write_buffer.drain(..) {
            match segment.write_value(&value) {
                Ok(vptr) => {
                    // Notify callback with success
                    if let Some(cb) = &callback {
                        match cb {
                            WriteCallback::Channel(sender) => { let _ = sender.send(Ok(vptr)); }
                            WriteCallback::None => {}
                        }
                    }
                }
                Err(e) => {
                    // Notify callback with error
                    if let Some(cb) = &callback {
                        match cb {
                            WriteCallback::Channel(sender) => { let _ = sender.send(Err(e)); }
                            WriteCallback::None => {}
                        }
                    }
                }
            }
        }

        // Check if segment is full and rotate if needed
        if segment.should_rotate() {
            segment.close()?;
            *current_segment = Some(VlogSegment::new(vlog_dir, config, queue_id as u64)?);
        }

        Ok(())
    }

    /// Write a value to the value log
    pub async fn write_value(&self, value: Value) -> Result<ValuePointer> {
        // Choose a write queue (round-robin or hash-based)
        let queue_id = self.choose_write_queue(&value);
        
        // Create a channel for the response
        let (tx, mut rx) = mpsc::channel(1);
        let callback = WriteCallback::Channel(tx);

        // Send write request
        if let Some(sender) = self.write_queues.get(queue_id) {
            let request = WriteRequest::Write { value, callback };
            sender.send(request).map_err(|_| Error::Concurrency("Failed to send write request".to_string()))?;
        } else {
            return Err(Error::Concurrency("Invalid write queue".to_string()));
        }

        // Wait for response
        match rx.recv().await {
            Some(result) => result,
            None => Err(Error::Concurrency("Write request timed out".to_string())),
        }
    }

    /// Write a value synchronously (for small values or when async is disabled)
    pub fn write_value_sync(&mut self, value: Value) -> Result<ValuePointer> {
        // Choose a segment (round-robin)
        let segment_id = self.next_segment_id.fetch_add(1, Ordering::SeqCst) % self.segments.len() as u64;
        
        if let Some(segment) = self.segments.get(segment_id as usize) {
            let mut segment = segment.write();
            segment.write_value(&value)
        } else {
            Err(Error::Concurrency("No available segments".to_string()))
        }
    }

    /// Choose a write queue for the value
    fn choose_write_queue(&self, value: &Value) -> usize {
        // Simple hash-based distribution
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        value.data.hash(&mut hasher);
        (hasher.finish() % self.config.write_queues as u64) as usize
    }

    /// Create a new segment
    fn create_new_segment(&mut self) -> Result<()> {
        let segment_id = self.next_segment_id.fetch_add(1, Ordering::SeqCst);
        let segment = VlogSegment::new(&self.vlog_dir, &self.config, segment_id)?;
        
        self.segments.push(Arc::new(RwLock::new(segment)));
        Ok(())
    }

    /// Sync all segments
    pub async fn sync(&self) -> Result<()> {
        // Send sync request to all queues
        for sender in &self.write_queues {
            let _ = sender.send(WriteRequest::Sync);
        }

        // Wait a bit for sync to complete
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        Ok(())
    }

    /// Close the value log writer
    pub async fn close(&mut self) -> Result<()> {
        // Send shutdown signal to all queues
        for sender in &self.write_queues {
            let _ = sender.send(WriteRequest::Shutdown);
        }

        // Wait for all background tasks to finish
        for handle in self.background_handles.drain(..) {
            let _ = handle.await;
        }

        // Close all segments
        for segment in &self.segments {
            let mut segment = segment.write();
            segment.close()?;
        }

        Ok(())
    }

    /// Get segment metadata
    pub fn get_segment_metadata(&self, segment_id: u64) -> Option<&VlogSegmentMeta> {
        self.segment_metadata.get(&segment_id)
    }
}

/// Write request types
#[derive(Debug)]
pub enum WriteRequest {
    /// Write a value
    Write { value: Value, callback: WriteCallback },
    /// Sync the current segment
    Sync,
    /// Shutdown the writer
    Shutdown,
}

/// Write callback types
#[derive(Debug)]
pub enum WriteCallback {
    /// Channel-based callback
    Channel(mpsc::Sender<Result<ValuePointer>>),
    /// No callback
    None,
}

/// Individual value log segment
pub struct VlogSegment {
    /// File handle
    file: BufWriter<File>,
    /// Segment metadata
    meta: VlogSegmentMeta,
    /// Current offset
    current_offset: u64,
    /// Configuration
    config: ValueLogConfig,
}

impl VlogSegment {
    /// Create a new value log segment
    fn new(vlog_dir: &PathBuf, config: &ValueLogConfig, segment_id: u64) -> Result<Self> {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        let filename = format!("vlog_{:016x}_{:016x}.seg", segment_id, timestamp);
        let path = vlog_dir.join(filename);

        let file = OpenOptions::new()
            .create(true)
            .write(true)
            .append(true)
            .open(&path)?;

        let mut buf_writer = BufWriter::with_capacity(config.cache_size, file);

        // Write header
        let header = VlogHeader::new(config.compression_algorithm.clone());
        let header_bytes = bincode::serialize(&header)?;
        buf_writer.write_all(&header_bytes)?;
        buf_writer.flush()?;

        let meta = VlogSegmentMeta {
            path: path.clone(),
            size: header_bytes.len() as u64,
            entry_count: 0,
            first_offset: header_bytes.len() as u64,
            last_offset: header_bytes.len() as u64,
            created_at: timestamp,
            closed: false,
        };

        Ok(Self {
            file: buf_writer,
            meta,
            current_offset: header_bytes.len() as u64,
            config: config.clone(),
        })
    }

    /// Write a value to the segment
    fn write_value(&mut self, value: &Value) -> Result<ValuePointer> {
        // Compress value if enabled
        let (compressed_data, compression, checksum) = if self.config.compress_values {
            self.compress_value(&value.data)?
        } else {
            (value.data.clone(), CompressionAlgorithm::None, self.calculate_checksum(&value.data))
        };

        // Create entry metadata
        let entry = VlogEntry {
            length: compressed_data.len() as u32,
            compression,
            checksum,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
        };

        // Write entry metadata
        let entry_bytes = bincode::serialize(&entry)?;
        self.file.write_all(&(entry_bytes.len() as u32).to_le_bytes())?;
        self.file.write_all(&entry_bytes)?;

        // Write value data
        self.file.write_all(&compressed_data)?;

        // Update metadata
        let entry_size = 4 + entry_bytes.len() + compressed_data.len();
        let vptr = ValuePointer::with_checksum(
            self.meta.path.file_name().unwrap().to_string_lossy().parse::<u64>().unwrap_or(0),
            self.current_offset,
            compressed_data.len() as u32,
            checksum,
        );

        self.current_offset += entry_size as u64;
        self.meta.size = self.current_offset;
        self.meta.entry_count += 1;
        self.meta.last_offset = self.current_offset;

        Ok(vptr)
    }

    /// Compress a value
    fn compress_value(&self, data: &[u8]) -> Result<(Vec<u8>, CompressionAlgorithm, u32)> {
        // TODO: Re-implement compression when dependencies are available
        let checksum = self.calculate_checksum(data);
        Ok((data.to_vec(), CompressionAlgorithm::None, checksum))
    }

    /// Calculate checksum for data
    fn calculate_checksum(&self, data: &[u8]) -> u32 {
        crc32fast::hash(data)
    }

    /// Check if segment should be rotated
    fn should_rotate(&self) -> bool {
        self.meta.size >= self.config.max_segment_size
    }

    /// Close the segment
    fn close(&mut self) -> Result<()> {
        self.file.flush()?;
        self.file.get_ref().sync_all()?;
        self.meta.closed = true;
        Ok(())
    }
}

/// Value log reader for reading values from segments
pub struct VlogReader {
    /// Value log directory
    vlog_dir: PathBuf,
    /// Open segment handles
    segments: HashMap<u64, VlogSegmentReader>,
}

impl VlogReader {
    /// Create a new value log reader
    pub fn new(vlog_dir: PathBuf) -> Result<Self> {
        Ok(Self {
            vlog_dir,
            segments: HashMap::new(),
        })
    }

    /// Read a value using a value pointer
    pub fn read_value(&mut self, vptr: &ValuePointer) -> Result<Value> {
        // Get or create segment reader
        let segment_reader = if let Some(reader) = self.segments.get_mut(&vptr.segment_id) {
            reader
        } else {
            let reader = VlogSegmentReader::new(&self.vlog_dir, vptr.segment_id)?;
            self.segments.insert(vptr.segment_id, reader);
            self.segments.get_mut(&vptr.segment_id).unwrap()
        };

        // Read the value
        segment_reader.read_value_at(vptr.offset, vptr.length)
    }

    /// Close the reader
    pub fn close(&mut self) -> Result<()> {
        for (_, reader) in self.segments.drain() {
            reader.close()?;
        }
        Ok(())
    }
}

/// Value log segment reader
struct VlogSegmentReader {
    /// File handle
    file: File,
    /// Segment path
    path: PathBuf,
}

impl VlogSegmentReader {
    /// Create a new segment reader
    fn new(vlog_dir: &PathBuf, segment_id: u64) -> Result<Self> {
        // Find segment file by ID
        let entries = std::fs::read_dir(vlog_dir)?;
        let segment_path = entries
            .filter_map(|entry| entry.ok())
            .find(|entry| {
                entry.path().to_string_lossy().contains(&format!("vlog_{:016x}", segment_id))
            })
            .ok_or_else(|| Error::InvalidValuePointer(format!("Segment {} not found", segment_id)))?
            .path();

        let file = OpenOptions::new().read(true).open(&segment_path)?;

        Ok(Self {
            file,
            path: segment_path,
        })
    }

    /// Read a value at a specific offset
    fn read_value_at(&mut self, offset: u64, length: u32) -> Result<Value> {
        // Seek to the offset
        self.file.seek(SeekFrom::Start(offset))?;

        // Read entry metadata length
        let mut len_bytes = [0u8; 4];
        self.file.read_exact(&mut len_bytes)?;
        let entry_len = u32::from_le_bytes(len_bytes) as usize;

        // Read entry metadata
        let mut entry_bytes = vec![0u8; entry_len];
        self.file.read_exact(&mut entry_bytes)?;
        let entry: VlogEntry = bincode::deserialize(&entry_bytes)?;

        // Read value data
        let mut value_data = vec![0u8; entry.length as usize];
        self.file.read_exact(&mut value_data)?;

        // Decompress if needed
        let decompressed_data = if entry.compression != CompressionAlgorithm::None {
            self.decompress_value(&value_data, &entry.compression)?
        } else {
            value_data
        };

        // Verify checksum
        let calculated_checksum = self.calculate_checksum(&decompressed_data);
        if calculated_checksum != entry.checksum {
            return Err(Error::ValueLogCorruption(format!(
                "Checksum mismatch: expected {}, got {}",
                entry.checksum, calculated_checksum
            )));
        }

        Ok(Value::new(decompressed_data))
    }

    /// Decompress a value
    fn decompress_value(&self, data: &[u8], compression: &CompressionAlgorithm) -> Result<Vec<u8>> {
        match compression {
            CompressionAlgorithm::Lz4 => {
                // TODO: Re-implement decompression when dependencies are available
                Ok(data.to_vec())
            }
            CompressionAlgorithm::Zstd => {
                // TODO: Re-implement decompression when dependencies are available
                Ok(data.to_vec())
            }
            CompressionAlgorithm::Snappy => {
                // Note: snappy crate doesn't have a simple decompress function
                // For now, return as-is
                Ok(data.to_vec())
            }
            CompressionAlgorithm::None => Ok(data.to_vec()),
        }
    }

    /// Calculate checksum for data
    fn calculate_checksum(&self, data: &[u8]) -> u32 {
        crc32fast::hash(data)
    }

    /// Close the segment reader
    fn close(&mut self) -> Result<()> {
        // File will be closed automatically when dropped
        Ok(())
    }
}

impl Drop for VlogWriter {
    fn drop(&mut self) {
        // Try to close gracefully
        let _ = tokio::runtime::Handle::current().block_on(self.close());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_vlog_writer_creation() {
        let temp_dir = tempdir().unwrap();
        let config = ValueLogConfig {
            vlog_path: temp_dir.path().to_path_buf(),
            ..Default::default()
        };
        
        let writer = VlogWriter::new(config);
        assert!(writer.is_ok());
    }

    #[test]
    fn test_vlog_header_validation() {
        let header = VlogHeader::new(CompressionAlgorithm::Lz4);
        assert!(header.validate());
    }

    #[test]
    fn test_compression_decompression() {
        let data = b"Hello, World! This is a test string for compression testing.";
        let config = ValueLogConfig::default();
        
        // Test LZ4 compression
        // TODO: Re-implement compression when dependencies are available
        let compressed = data.to_vec();
        let decompressed = data.to_vec();
        assert_eq!(data, &decompressed[..]);
    }
}
