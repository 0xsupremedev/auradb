use crate::{error::Result, storage::{Key, Value, Batch, Range}};
use crate::config::Config;
use std::path::PathBuf;
use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;

/// Main engine trait defining the core KV operations
#[async_trait::async_trait]
pub trait Engine: Send + Sync {
    /// Put a key-value pair
    async fn put(&self, key: Key, value: Value) -> Result<()>;
    
    /// Get a value by key
    async fn get(&self, key: &Key) -> Result<Option<Value>>;
    
    /// Delete a key
    async fn delete(&self, key: &Key) -> Result<()>;
    
    /// Scan a range of keys
    async fn scan(&self, range: Range) -> Result<Vec<(Key, Value)>>;
    
    /// Write a batch of operations
    async fn write_batch(&self, batch: &Batch) -> Result<()>;
    
    /// Create a snapshot
    async fn snapshot(&self) -> Result<Snapshot>;
    
    /// Close the engine
    async fn close(&self) -> Result<()>;
}

/// Engine builder for easy configuration
pub struct EngineBuilder {
    config: Config,
}

impl EngineBuilder {
    /// Create a new engine builder
    pub fn new() -> Self {
        Self {
            config: Config::default(),
        }
    }
    
    /// Set the database path
    pub fn path<P: Into<PathBuf>>(mut self, path: P) -> Self {
        self.config.db_path = path.into();
        self
    }
    
    /// Build the engine
    pub fn build(self) -> Result<AuraEngine> {
        AuraEngine::new(self.config)
    }
}

impl Default for EngineBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Main AuraDB engine implementation
pub struct AuraEngine {
    /// Engine configuration
    config: Config,
    /// In-memory storage (simplified for now)
    storage: Arc<RwLock<HashMap<Vec<u8>, Vec<u8>>>>,
    /// Engine status
    closed: Arc<RwLock<bool>>,
}

impl AuraEngine {
    /// Create a new engine instance
    pub fn new(config: Config) -> Result<Self> {
        // Create directories
        std::fs::create_dir_all(&config.db_path)
            .map_err(|e| crate::error::Error::Io(e))?;
        
        // Create WAL and value log directories if they don't exist
        std::fs::create_dir_all(&config.wal.wal_path)
            .map_err(|e| crate::error::Error::Io(e))?;
        std::fs::create_dir_all(&config.value_log.vlog_path)
            .map_err(|e| crate::error::Error::Io(e))?;
        
        Ok(Self {
            config,
            storage: Arc::new(RwLock::new(HashMap::new())),
            closed: Arc::new(RwLock::new(false)),
        })
    }
    
    /// Put a string key-value pair (convenience method)
    pub fn put_str(&self, key: &str, value: &str) -> Result<()> {
        let key = Key::new(key.as_bytes().to_vec());
        let value = Value::new(value.as_bytes().to_vec());
        
        let mut storage = self.storage.write();
        storage.insert(key.data, value.data);
        Ok(())
    }
    
    /// Get a string value by key (convenience method)
    pub fn get_str(&self, key: &str) -> Result<Option<String>> {
        let key = Key::new(key.as_bytes().to_vec());
        
        let storage = self.storage.read();
        if let Some(value_data) = storage.get(&key.data) {
            Ok(Some(String::from_utf8_lossy(value_data).to_string()))
        } else {
            Ok(None)
        }
    }
    
    /// Delete a string key (convenience method)
    pub fn delete_str(&self, key: &str) -> Result<()> {
        let key = Key::new(key.as_bytes().to_vec());
        
        let mut storage = self.storage.write();
        storage.remove(&key.data);
        Ok(())
    }
    
    /// Scan string keys in a range (convenience method)
    pub fn scan_str(&self, start: &str, end: &str) -> Result<Vec<(String, String)>> {
        let start_key = Key::new(start.as_bytes().to_vec());
        let end_key = Key::new(end.as_bytes().to_vec());
        
        let storage = self.storage.read();
        let mut results = Vec::new();
        
        for (key_data, value_data) in storage.iter() {
            if key_data >= &start_key.data && key_data <= &end_key.data {
                let key = String::from_utf8_lossy(key_data).to_string();
                let value = String::from_utf8_lossy(value_data).to_string();
                results.push((key, value));
            }
        }
        
        Ok(results)
    }
    
    /// Write a batch of key-value pairs
    pub fn write_batch(&self, batch: &[(Vec<u8>, Vec<u8>)]) -> Result<()> {
        let mut storage = self.storage.write();
        for (key, value) in batch {
            storage.insert(key.clone(), value.clone());
        }
        Ok(())
    }

    /// Put a key-value pair using Vec<u8> (for benchmarks)
    pub fn put_bytes(&self, key: &[u8], value: &[u8]) -> Result<()> {
        let mut storage = self.storage.write();
        storage.insert(key.to_vec(), value.to_vec());
        Ok(())
    }

    /// Get a value by key using Vec<u8> (for benchmarks)
    pub fn get_bytes(&self, key: &[u8]) -> Result<Option<Vec<u8>>> {
        let storage = self.storage.read();
        Ok(storage.get(key).cloned())
    }
}

#[async_trait::async_trait]
impl Engine for AuraEngine {
    async fn put(&self, key: Key, value: Value) -> Result<()> {
        let mut storage = self.storage.write();
        storage.insert(key.data, value.data);
        Ok(())
    }
    
    async fn get(&self, key: &Key) -> Result<Option<Value>> {
        let storage = self.storage.read();
        if let Some(value_data) = storage.get(&key.data) {
            Ok(Some(Value::new(value_data.clone())))
        } else {
            Ok(None)
        }
    }
    
    async fn delete(&self, key: &Key) -> Result<()> {
        let mut storage = self.storage.write();
        storage.remove(&key.data);
        Ok(())
    }
    
    async fn scan(&self, range: Range) -> Result<Vec<(Key, Value)>> {
        let storage = self.storage.read();
        let mut results = Vec::new();
        
        for (key_data, value_data) in storage.iter() {
            if key_data >= &range.start.data && key_data <= &range.end.data {
                let key = Key::new(key_data.clone());
                let value = Value::new(value_data.clone());
                results.push((key, value));
            }
        }
        
        Ok(results)
    }
    
    async fn write_batch(&self, batch: &Batch) -> Result<()> {
        let mut storage = self.storage.write();
        
        for entry in &batch.operations {
            match entry.op_type {
                crate::storage::OpType::Put => {
                    if let Some(value) = &entry.value {
                        storage.insert(entry.key.data.clone(), value.data.clone());
                    }
                }
                crate::storage::OpType::Delete => {
                    storage.remove(&entry.key.data);
                }
                crate::storage::OpType::Merge => {
                    // For now, treat merge as put
                    if let Some(value) = &entry.value {
                        storage.insert(entry.key.data.clone(), value.data.clone());
                    }
                }
            }
        }
        
        Ok(())
    }
    
    async fn snapshot(&self) -> Result<Snapshot> {
        let storage = self.storage.read();
        let mut snapshot_data = HashMap::new();
        
        for (key, value) in storage.iter() {
            snapshot_data.insert(key.clone(), value.clone());
        }
        
        Ok(Snapshot {
            data: snapshot_data,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
        })
    }
    
    async fn close(&self) -> Result<()> {
        let mut closed = self.closed.write();
        *closed = true;
        Ok(())
    }
}

/// Database snapshot
pub struct Snapshot {
    /// Snapshot data
    pub data: HashMap<Vec<u8>, Vec<u8>>,
    /// Timestamp when snapshot was created
    pub timestamp: u64,
}

/// Engine options
#[derive(Debug, Clone)]
pub struct Options {
    /// Database path
    pub path: PathBuf,
    /// Whether to create directories if they don't exist
    pub create_if_missing: bool,
    /// Whether to error if database already exists
    pub error_if_exists: bool,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            path: PathBuf::from("./auradb_data"),
            create_if_missing: true,
            error_if_exists: false,
        }
    }
}
