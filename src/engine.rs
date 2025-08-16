//! Main engine module for AuraDB
//! 
//! This module provides the core storage engine implementation and public API.

pub use crate::api::{AuraEngine, Engine, EngineBuilder, Options, Snapshot};

/// Re-export commonly used types
pub mod types {
    pub use crate::storage::{Key, Value, ValuePointer, Entry, Batch, Range};
    pub use crate::config::Config;
    pub use crate::error::{Error, Result};
}

/// Engine statistics and metrics
#[derive(Debug, Clone)]
pub struct EngineStats {
    /// Total number of operations
    pub total_operations: u64,
    /// Number of reads
    pub reads: u64,
    /// Number of writes
    pub writes: u64,
    /// Number of deletes
    pub deletes: u64,
    /// Current memtable size in bytes
    pub memtable_size: usize,
    /// Current WAL size in bytes
    pub wal_size: usize,
    /// Current value log size in bytes
    pub vlog_size: usize,
    /// Number of SST files
    pub sst_files: u64,
    /// Write amplification
    pub write_amplification: f64,
    /// Read amplification
    pub read_amplification: f64,
}

impl Default for EngineStats {
    fn default() -> Self {
        Self {
            total_operations: 0,
            reads: 0,
            writes: 0,
            deletes: 0,
            memtable_size: 0,
            wal_size: 0,
            vlog_size: 0,
            sst_files: 0,
            write_amplification: 1.0,
            read_amplification: 1.0,
        }
    }
}

/// Engine status
#[derive(Debug, Clone)]
pub enum EngineStatus {
    /// Engine is starting up
    Starting,
    /// Engine is running normally
    Running,
    /// Engine is in read-only mode
    ReadOnly,
    /// Engine is shutting down
    ShuttingDown,
    /// Engine has encountered an error
    Error(String),
}

/// Engine information
#[derive(Debug, Clone)]
pub struct EngineInfo {
    /// Engine version
    pub version: String,
    /// Engine status
    pub status: EngineStatus,
    /// Engine statistics
    pub stats: EngineStats,
    /// Configuration summary
    pub config_summary: String,
}

impl EngineInfo {
    /// Create new engine info
    pub fn new() -> Self {
        Self {
            version: env!("CARGO_PKG_VERSION").to_string(),
            status: EngineStatus::Starting,
            stats: EngineStats::default(),
            config_summary: "Default configuration".to_string(),
        }
    }
}

/// Engine health check result
#[derive(Debug, Clone)]
pub struct HealthCheck {
    /// Whether the engine is healthy
    pub healthy: bool,
    /// Health check message
    pub message: String,
    /// Timestamp of the health check
    pub timestamp: u64,
}

impl HealthCheck {
    /// Create a new health check
    pub fn new(healthy: bool, message: String) -> Self {
        Self {
            healthy,
            message,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
        }
    }
    
    /// Create a healthy health check
    pub fn healthy(message: String) -> Self {
        Self::new(true, message)
    }
    
    /// Create an unhealthy health check
    pub fn unhealthy(message: String) -> Self {
        Self::new(false, message)
    }
}

/// Engine extension trait for additional functionality
#[async_trait::async_trait]
pub trait EngineExt: Engine {
    /// Get engine information
    async fn info(&self) -> Result<EngineInfo>;
    
    /// Get engine statistics
    async fn stats(&self) -> Result<EngineStats>;
    
    /// Perform health check
    async fn health_check(&self) -> Result<HealthCheck>;
    
    /// Compact the database
    async fn compact(&self) -> Result<()>;
    
    /// Backup the database
    async fn backup(&self, path: &std::path::Path) -> Result<()>;
    
    /// Restore from backup
    async fn restore(&self, path: &std::path::Path) -> Result<()>;
}

#[async_trait::async_trait]
impl EngineExt for AuraEngine {
    async fn info(&self) -> Result<EngineInfo> {
        let mut info = EngineInfo::new();
        info.status = EngineStatus::Running;
        info.config_summary = format!("DB: {:?}, WAL: {:?}, VLog: {:?}", 
            self.config.db_path, 
            self.config.wal.wal_path, 
            self.config.value_log.vlog_path
        );
        Ok(info)
    }
    
    async fn stats(&self) -> Result<EngineStats> {
        let mut stats = EngineStats::default();
        
        // Get memtable stats
        let memtable = self.memtable.read();
        stats.memtable_size = memtable.memory_usage();
        
        // TODO: Get WAL, VLog, and SST stats
        // For now, return default values
        
        Ok(stats)
    }
    
    async fn health_check(&self) -> Result<HealthCheck> {
        // Simple health check - verify we can perform basic operations
        if *self.closed.read() {
            return Ok(HealthCheck::unhealthy("Engine is closed".to_string()));
        }
        
        // Check if memtable is accessible
        let memtable = self.memtable.read();
        if memtable.is_empty() {
            // This is fine - empty memtable is valid
        }
        
        Ok(HealthCheck::healthy("Engine is healthy".to_string()))
    }
    
    async fn compact(&self) -> Result<()> {
        // TODO: Implement compaction
        // For now, just flush the memtable
        self.flush_memtable().await?;
        Ok(())
    }
    
    async fn backup(&self, _path: &std::path::Path) -> Result<()> {
        // TODO: Implement backup functionality
        Err(crate::error::Error::Unknown("Backup not implemented yet".to_string()))
    }
    
    async fn restore(&self, _path: &std::path::Path) -> Result<()> {
        // TODO: Implement restore functionality
        Err(crate::error::Error::Unknown("Restore not implemented yet".to_string()))
    }
}

/// Engine builder with additional configuration options
pub struct AdvancedEngineBuilder {
    config: crate::config::Config,
}

impl AdvancedEngineBuilder {
    /// Create a new advanced engine builder
    pub fn new() -> Self {
        Self {
            config: crate::config::Config::default(),
        }
    }
    
    /// Set the database path
    pub fn with_db_path(mut self, path: std::path::PathBuf) -> Self {
        self.config.db_path = path;
        self
    }
    
    /// Set WAL configuration
    pub fn with_wal_config(mut self, wal_config: crate::config::WalConfig) -> Self {
        self.config.wal = wal_config;
        self
    }
    
    /// Set value log configuration
    pub fn with_vlog_config(mut self, vlog_config: crate::config::ValueLogConfig) -> Self {
        self.config.value_log = vlog_config;
        self
    }
    
    /// Set memtable configuration
    pub fn with_memtable_config(mut self, memtable_config: crate::config::MemtableConfig) -> Self {
        self.config.memtable = memtable_config;
        self
    }
    
    /// Set SST configuration
    pub fn with_sst_config(mut self, sst_config: crate::config::SstConfig) -> Self {
        self.config.sst = sst_config;
        self
    }
    
    /// Set compaction configuration
    pub fn with_compaction_config(mut self, compaction_config: crate::config::CompactionConfig) -> Self {
        self.config.compaction = compaction_config;
        self
    }
    
    /// Set cache configuration
    pub fn with_cache_config(mut self, cache_config: crate::config::CacheConfig) -> Self {
        self.config.cache = cache_config;
        self
    }
    
    /// Set learned index configuration
    pub fn with_learned_index_config(mut self, learned_index_config: crate::config::LearnedIndexConfig) -> Self {
        self.config.learned_index = learned_index_config;
        self
    }
    
    /// Set RL agent configuration
    pub fn with_rl_agent_config(mut self, rl_agent_config: crate::config::RlAgentConfig) -> Self {
        self.config.rl_agent = rl_agent_config;
        self
    }
    
    /// Set performance configuration
    pub fn with_performance_config(mut self, performance_config: crate::config::PerformanceConfig) -> Self {
        self.config.performance = performance_config;
        self
    }
    
    /// Build the engine
    pub async fn build(self) -> Result<AuraEngine> {
        AuraEngine::new(self.config).await
    }
}

impl Default for AdvancedEngineBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Convenience function to create a simple engine
pub async fn create_engine(db_path: std::path::PathBuf) -> Result<AuraEngine> {
    EngineBuilder::new()
        .with_db_path(db_path)
        .build()
        .await
}

/// Convenience function to create an engine with custom configuration
pub async fn create_engine_with_config(config: crate::config::Config) -> Result<AuraEngine> {
    AuraEngine::new(config).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    
    #[tokio::test]
    async fn test_engine_ext() {
        let temp_dir = tempdir().unwrap();
        let engine = create_engine(temp_dir.path().to_path_buf()).await.unwrap();
        
        // Test info
        let info = engine.info().await.unwrap();
        assert_eq!(info.version, env!("CARGO_PKG_VERSION"));
        
        // Test stats
        let stats = engine.stats().await.unwrap();
        assert_eq!(stats.total_operations, 0);
        
        // Test health check
        let health = engine.health_check().await.unwrap();
        assert!(health.healthy);
        
        engine.close().await.unwrap();
    }
    
    #[tokio::test]
    async fn test_advanced_builder() {
        let temp_dir = tempdir().unwrap();
        let engine = AdvancedEngineBuilder::new()
            .with_db_path(temp_dir.path().to_path_buf())
            .build()
            .await
            .unwrap();
        
        assert!(engine.info().await.is_ok());
        engine.close().await.unwrap();
    }
}
