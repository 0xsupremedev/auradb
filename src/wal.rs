use crate::config::{WalConfig, WalSyncPolicy};
use crate::error::{Error, Result};
use crate::storage::{Entry, ValuePointer};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::time::{self, Duration};
use tracing::{debug, error, info, warn};

/// WAL record types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WalRecord {
    /// Put operation with inline value
    Put {
        key: Vec<u8>,
        value: Vec<u8>,
        sequence: u64,
        timestamp: u64,
    },
    /// Put operation with value pointer (WAL-time KV separation)
    PutPointer {
        key: Vec<u8>,
        value_pointer: ValuePointer,
        sequence: u64,
        timestamp: u64,
    },
    /// Delete operation
    Delete {
        key: Vec<u8>,
        sequence: u64,
        timestamp: u64,
    },
    /// Batch operation
    Batch {
        operations: Vec<WalRecord>,
        sequence: u64,
        timestamp: u64,
    },
}

/// WAL file header
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalHeader {
    /// Magic number for WAL files
    pub magic: [u8; 8],
    /// Version number
    pub version: u32,
    /// File creation timestamp
    pub created_at: u64,
    /// Checksum of the header
    pub checksum: u32,
}

impl WalHeader {
    const MAGIC: [u8; 8] = [0x41, 0x55, 0x52, 0x41, 0x44, 0x42, 0x57, 0x41]; // "AURADBWA"
    const VERSION: u32 = 1;

    /// Create a new WAL header
    pub fn new() -> Self {
        let created_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        Self {
            magic: Self::MAGIC,
            version: Self::VERSION,
            created_at,
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
        hasher.finalize()
    }

    /// Validate the header
    pub fn validate(&self) -> bool {
        self.magic == Self::MAGIC
            && self.version == Self::VERSION
            && self.checksum == self.calculate_checksum()
    }
}

/// WAL file metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalFileMeta {
    /// File path
    pub path: PathBuf,
    /// File size in bytes
    pub size: u64,
    /// First sequence number in the file
    pub first_sequence: u64,
    /// Last sequence number in the file
    pub last_sequence: u64,
    /// File creation timestamp
    pub created_at: u64,
    /// Whether the file is closed
    pub closed: bool,
}

/// WAL writer that handles writing records to WAL files
pub struct WalWriter {
    /// Current WAL file
    current_file: Option<WalFile>,
    /// WAL configuration
    config: WalConfig,
    /// Current sequence number
    sequence: AtomicU64,
    /// WAL directory path
    wal_dir: PathBuf,
    /// Async write channel
    async_sender: Option<mpsc::UnboundedSender<AsyncWriteRequest>>,
    /// Background task handle
    background_handle: Option<tokio::task::JoinHandle<()>>,
}

impl WalWriter {
    /// Create a new WAL writer
    pub fn new(config: WalConfig) -> Result<Self> {
        let wal_dir = config.wal_path.clone();
        std::fs::create_dir_all(&wal_dir)?;

        let mut writer = Self {
            current_file: None,
            config,
            sequence: AtomicU64::new(0),
            wal_dir,
            async_sender: None,
            background_handle: None,
        };

        if writer.config.async_writes {
            writer.start_async_writer()?;
        }

        writer.rotate_file()?;
        Ok(writer)
    }

    /// Start the async writer background task
    fn start_async_writer(&mut self) -> Result<()> {
        let (tx, mut rx) = mpsc::unbounded_channel();
        self.async_sender = Some(tx);

        let wal_dir = self.wal_dir.clone();
        let config = self.config.clone();
        let handle = tokio::spawn(async move {
            let mut current_file = None;
            let mut write_buffer = Vec::new();

            while let Some(request) = rx.recv().await {
                match request {
                    AsyncWriteRequest::Write(record) => {
                        write_buffer.push(record);
                        
                        // Flush if buffer is full or sync is requested
                        if write_buffer.len() >= 1000 {
                            if let Err(e) = Self::flush_records(&mut current_file, &wal_dir, &config, &mut write_buffer).await {
                                error!("Failed to flush WAL records: {}", e);
                            }
                        }
                    }
                    AsyncWriteRequest::Sync => {
                        if let Err(e) = Self::flush_records(&mut current_file, &wal_dir, &config, &mut write_buffer).await {
                            error!("Failed to sync WAL records: {}", e);
                        }
                    }
                    AsyncWriteRequest::Shutdown => break,
                }
            }
        });

        self.background_handle = Some(handle);
        Ok(())
    }

    /// Flush records to WAL file (async helper)
    async fn flush_records(
        current_file: &mut Option<WalFile>,
        wal_dir: &PathBuf,
        config: &WalConfig,
        records: &mut Vec<WalRecord>,
    ) -> Result<()> {
        if records.is_empty() {
            return Ok(());
        }

        // Ensure we have a current file
        if current_file.is_none() {
            *current_file = Some(WalFile::new(wal_dir, config)?);
        }

        let file = current_file.as_mut().unwrap();
        
        // Write all records
        for record in records.drain(..) {
            file.write_record(&record)?;
        }

        // Sync based on policy
        match config.sync_policy {
            WalSyncPolicy::EveryWrite => file.sync()?,
            WalSyncPolicy::EveryNWrites(n) if file.record_count() % n == 0 => file.sync()?,
            WalSyncPolicy::EveryNMs(ms) => {
                // This is simplified - in practice you'd want more sophisticated timing
                time::sleep(Duration::from_millis(ms)).await;
                file.sync()?;
            }
            _ => {}
        }

        Ok(())
    }

    /// Write a record to the WAL
    pub fn write_record(&mut self, record: &WalRecord) -> Result<u64> {
        let sequence = self.sequence.fetch_add(1, Ordering::SeqCst);

        if self.config.async_writes {
            if let Some(sender) = &self.async_sender {
                let _ = sender.send(AsyncWriteRequest::Write(record.clone()));
            }
        } else {
            self.ensure_current_file()?;
            self.current_file.as_mut().unwrap().write_record(record)?;
            
            // Handle sync policy
            match self.config.sync_policy {
                WalSyncPolicy::EveryWrite => self.sync()?,
                WalSyncPolicy::EveryNWrites(n) if sequence % n == 0 => self.sync()?,
                _ => {}
            }
        }

        Ok(sequence)
    }

    /// Write a batch of operations
    pub fn write_batch(&mut self, entries: &[Entry]) -> Result<u64> {
        let sequence = self.sequence.fetch_add(1, Ordering::SeqCst);
        
        let records: Vec<WalRecord> = entries
            .iter()
            .map(|entry| {
                if let Some(value) = &entry.value {
                    WalRecord::Put {
                        key: entry.key.data.clone(),
                        value: value.data.clone(),
                        sequence: entry.sequence,
                        timestamp: entry.timestamp,
                    }
                } else if let Some(vptr) = &entry.value_pointer {
                    WalRecord::PutPointer {
                        key: entry.key.data.clone(),
                        value_pointer: vptr.clone(),
                        sequence: entry.sequence,
                        timestamp: entry.timestamp,
                    }
                } else {
                    WalRecord::Delete {
                        key: entry.key.data.clone(),
                        sequence: entry.sequence,
                        timestamp: entry.timestamp,
                    }
                }
            })
            .collect();

        let batch_record = WalRecord::Batch {
            operations: records,
            sequence,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
        };

        self.write_record(&batch_record)?;
        Ok(sequence)
    }

    /// Ensure we have a current WAL file
    fn ensure_current_file(&mut self) -> Result<()> {
        if self.current_file.is_none() || self.should_rotate() {
            self.rotate_file()?;
        }
        Ok(())
    }

    /// Check if we should rotate to a new WAL file
    fn should_rotate(&self) -> bool {
        if let Some(file) = &self.current_file {
            file.size() >= self.config.max_file_size
        } else {
            true
        }
    }

    /// Rotate to a new WAL file
    fn rotate_file(&mut self) -> Result<()> {
        // Close current file if it exists
        if let Some(mut file) = self.current_file.take() {
            file.close()?;
        }

        // Create new file
        let file = WalFile::new(&self.wal_dir, &self.config)?;
        self.current_file = Some(file);
        
        info!("Rotated to new WAL file");
        Ok(())
    }

    /// Sync the current WAL file
    pub fn sync(&mut self) -> Result<()> {
        if let Some(file) = &mut self.current_file {
            file.sync()?;
        }
        Ok(())
    }

    /// Get the current sequence number
    pub fn current_sequence(&self) -> u64 {
        self.sequence.load(Ordering::SeqCst)
    }

    /// Close the WAL writer
    pub fn close(&mut self) -> Result<()> {
        // Send shutdown signal to async writer
        if let Some(sender) = &self.async_sender {
            let _ = sender.send(AsyncWriteRequest::Shutdown);
        }

        // Wait for background task to finish
        if let Some(handle) = self.background_handle.take() {
            // In a real implementation, you'd want to handle this more gracefully
            let _ = std::panic::catch_unwind(|| {
                // This is simplified - in practice you'd want proper shutdown coordination
            });
        }

        // Close current file
        if let Some(mut file) = self.current_file.take() {
            file.close()?;
        }

        Ok(())
    }
}

/// Async write request types
#[derive(Debug, Clone)]
pub enum AsyncWriteRequest {
    /// Write a record
    Write(WalRecord),
    /// Sync the current file
    Sync,
    /// Shutdown the async writer
    Shutdown,
}

/// Individual WAL file
struct WalFile {
    /// File handle
    file: BufWriter<File>,
    /// File metadata
    meta: WalFileMeta,
    /// Record count
    record_count: u64,
}

impl WalFile {
    /// Create a new WAL file
    fn new(wal_dir: &PathBuf, config: &WalConfig) -> Result<Self> {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        let filename = format!("wal_{:016x}.log", timestamp);
        let path = wal_dir.join(filename);

        let file = OpenOptions::new()
            .create(true)
            .write(true)
            .append(true)
            .open(&path)?;

        let mut buf_writer = BufWriter::with_capacity(config.buffer_size, file);

        // Write header
        let header = WalHeader::new();
        let header_bytes = bincode::serialize(&header)?;
        buf_writer.write_all(&header_bytes)?;
        buf_writer.flush()?;

        let meta = WalFileMeta {
            path: path.clone(),
            size: header_bytes.len() as u64,
            first_sequence: 0,
            last_sequence: 0,
            created_at: timestamp,
            closed: false,
        };

        Ok(Self {
            file: buf_writer,
            meta,
            record_count: 0,
        })
    }

    /// Write a record to the file
    fn write_record(&mut self, record: &WalRecord) -> Result<()> {
        let record_bytes = bincode::serialize(record)?;
        let record_len = record_bytes.len() as u32;
        
        // Write record length and data
        self.file.write_all(&record_len.to_le_bytes())?;
        self.file.write_all(&record_bytes)?;
        
        self.meta.size += 4 + record_bytes.len() as u64;
        self.record_count += 1;
        
        Ok(())
    }

    /// Sync the file to disk
    fn sync(&mut self) -> Result<()> {
        self.file.flush()?;
        self.file.get_ref().sync_all()?;
        Ok(())
    }

    /// Close the file
    fn close(&mut self) -> Result<()> {
        self.sync()?;
        self.meta.closed = true;
        Ok(())
    }

    /// Get the current file size
    fn size(&self) -> u64 {
        self.meta.size
    }

    /// Get the record count
    fn record_count(&self) -> u64 {
        self.record_count
    }
}

/// WAL reader for recovery
pub struct WalReader {
    /// WAL directory path
    wal_dir: PathBuf,
    /// Current file being read
    current_file: Option<WalFileReader>,
    /// File list to read
    files: VecDeque<PathBuf>,
}

impl WalReader {
    /// Create a new WAL reader
    pub fn new(wal_dir: PathBuf) -> Result<Self> {
        let mut files: Vec<PathBuf> = std::fs::read_dir(&wal_dir)?
            .filter_map(|entry| entry.ok())
            .filter(|entry| {
                entry.path().extension().map_or(false, |ext| ext == "log")
            })
            .map(|entry| entry.path())
            .collect();

        // Sort files by creation time (filename contains timestamp)
        files.sort();

        Ok(Self {
            wal_dir,
            current_file: None,
            files: files.into(),
        })
    }

    /// Read the next record from the WAL
    pub fn read_next(&mut self) -> Result<Option<WalRecord>> {
        loop {
            // Ensure we have a current file
            if self.current_file.is_none() {
                if let Some(file_path) = self.files.pop_front() {
                    self.current_file = Some(WalFileReader::new(file_path)?);
                } else {
                    return Ok(None); // No more files
                }
            }

            // Try to read from current file
            if let Some(file) = &mut self.current_file {
                match file.read_record()? {
                    Some(record) => return Ok(Some(record)),
                    None => {
                        // End of file, move to next
                        self.current_file = None;
                        continue;
                    }
                }
            }
        }
    }
}

/// WAL file reader for recovery
struct WalFileReader {
    /// File handle
    file: std::io::BufReader<File>,
    /// File path
    path: PathBuf,
}

impl WalFileReader {
    /// Create a new WAL file reader
    fn new(path: PathBuf) -> Result<Self> {
        let file = OpenOptions::new().read(true).open(&path)?;
        let reader = std::io::BufReader::new(file);

        Ok(Self { file: reader, path })
    }

    /// Read a record from the file
    fn read_record(&mut self) -> Result<Option<WalRecord>> {
        // Read record length
        let mut len_bytes = [0u8; 4];
        if self.file.read_exact(&mut len_bytes).is_err() {
            return Ok(None); // End of file
        }

        let record_len = u32::from_le_bytes(len_bytes) as usize;
        
        // Read record data
        let mut record_bytes = vec![0u8; record_len];
        self.file.read_exact(&mut record_bytes)?;
        
        // Deserialize record
        let record: WalRecord = bincode::deserialize(&record_bytes)?;
        Ok(Some(record))
    }
}

impl Drop for WalWriter {
    fn drop(&mut self) {
        let _ = self.close();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_wal_writer_creation() {
        let temp_dir = tempdir().unwrap();
        let config = WalConfig {
            wal_path: temp_dir.path().to_path_buf(),
            ..Default::default()
        };
        
        let writer = WalWriter::new(config);
        assert!(writer.is_ok());
    }

    #[test]
    fn test_wal_record_serialization() {
        let record = WalRecord::Put {
            key: b"test_key".to_vec(),
            value: b"test_value".to_vec(),
            sequence: 1,
            timestamp: 1234567890,
        };

        let serialized = bincode::serialize(&record).unwrap();
        let deserialized: WalRecord = bincode::deserialize(&serialized).unwrap();

        match deserialized {
            WalRecord::Put { key, value, sequence, timestamp } => {
                assert_eq!(key, b"test_key");
                assert_eq!(value, b"test_value");
                assert_eq!(sequence, 1);
                assert_eq!(timestamp, 1234567890);
            }
            _ => panic!("Unexpected record type"),
        }
    }
}
