use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Configuration for the AuraDB storage engine
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Database directory path
    pub db_path: PathBuf,
    
    /// WAL configuration
    pub wal: WalConfig,
    
    /// Value log configuration
    pub value_log: ValueLogConfig,
    
    /// Memtable configuration
    pub memtable: MemtableConfig,
    
    /// SST configuration
    pub sst: SstConfig,
    
    /// Compaction configuration
    pub compaction: CompactionConfig,
    
    /// Cache configuration
    pub cache: CacheConfig,
    
    /// Learned index configuration
    pub learned_index: LearnedIndexConfig,
    
    /// RL agent configuration
    pub rl_agent: RlAgentConfig,
    
    /// Performance tuning
    pub performance: PerformanceConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            db_path: PathBuf::from("./auradb_data"),
            wal: WalConfig::default(),
            value_log: ValueLogConfig::default(),
            memtable: MemtableConfig::default(),
            sst: SstConfig::default(),
            compaction: CompactionConfig::default(),
            cache: CacheConfig::default(),
            learned_index: LearnedIndexConfig::default(),
            rl_agent: RlAgentConfig::default(),
            performance: PerformanceConfig::default(),
        }
    }
}

/// WAL (Write-Ahead Log) configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalConfig {
    /// WAL directory path
    pub wal_path: PathBuf,
    /// Maximum WAL file size in bytes
    pub max_file_size: u64,
    /// Whether to use async WAL writes
    pub async_writes: bool,
    /// WAL sync policy
    pub sync_policy: WalSyncPolicy,
    /// WAL buffer size in bytes
    pub buffer_size: usize,
}

impl Default for WalConfig {
    fn default() -> Self {
        Self {
            wal_path: PathBuf::from("./auradb_data/wal"),
            max_file_size: 64 * 1024 * 1024, // 64MB
            async_writes: true,
            sync_policy: WalSyncPolicy::EveryWrite,
            buffer_size: 64 * 1024, // 64KB
        }
    }
}

/// WAL sync policy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WalSyncPolicy {
    /// Sync every write (safest, slowest)
    EveryWrite,
    /// Sync every N writes
    EveryNWrites(u64),
    /// Sync every N milliseconds
    EveryNMs(u64),
    /// Manual sync only
    Manual,
}

/// Value log configuration for WAL-time KV separation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValueLogConfig {
    /// Value log directory path
    pub vlog_path: PathBuf,
    /// Maximum segment size in bytes
    pub max_segment_size: u64,
    /// Value size threshold for separation (bytes)
    pub separation_threshold: usize,
    /// Number of parallel write queues
    pub write_queues: usize,
    /// Value log cache size in bytes
    pub cache_size: usize,
    /// Whether to compress values
    pub compress_values: bool,
    /// Compression algorithm
    pub compression_algorithm: CompressionAlgorithm,
}

impl Default for ValueLogConfig {
    fn default() -> Self {
        Self {
            vlog_path: PathBuf::from("./auradb_data/vlog"),
            max_segment_size: 256 * 1024 * 1024, // 256MB
            separation_threshold: 1024, // 1KB
            write_queues: 4,
            cache_size: 64 * 1024 * 1024, // 64MB
            compress_values: true,
            compression_algorithm: CompressionAlgorithm::Lz4,
        }
    }
}

/// Compression algorithms
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Copy)]
pub enum CompressionAlgorithm {
    /// No compression
    None,
    /// Lz4 compression (fast)
    Lz4,
    /// Zstandard compression (good compression ratio)
    Zstd,
    /// Snappy compression (balanced)
    Snappy,
}

/// Memtable configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemtableConfig {
    /// Maximum memtable size in bytes
    pub max_size: usize,
    /// Memtable implementation
    pub implementation: MemtableImpl,
    /// Number of memtables
    pub count: usize,
    /// Flush threshold (percentage of max_size)
    pub flush_threshold: f64,
}

impl Default for MemtableConfig {
    fn default() -> Self {
        Self {
            max_size: 64 * 1024 * 1024, // 64MB
            implementation: MemtableImpl::SkipList,
            count: 2,
            flush_threshold: 0.8, // 80%
        }
    }
}

/// Memtable implementation type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MemtableImpl {
    /// Skip list implementation
    SkipList,
    /// Adaptive Radix Tree (ART)
    Art,
    /// B-tree implementation
    BTree,
}

/// SST (Sorted String Table) configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SstConfig {
    /// SST directory path
    pub sst_path: PathBuf,
    /// Target file size in bytes
    pub target_file_size: u64,
    /// Block size in bytes
    pub block_size: usize,
    /// Whether to use Bloom filters
    pub use_bloom_filters: bool,
    /// Bloom filter bits per key
    pub bloom_bits_per_key: f64,
    /// Whether to use Ribbon filters
    pub use_ribbon_filters: bool,
    /// Compression algorithm for SST blocks
    pub compression: CompressionAlgorithm,
}

impl Default for SstConfig {
    fn default() -> Self {
        Self {
            sst_path: PathBuf::from("./auradb_data/sst"),
            target_file_size: 64 * 1024 * 1024, // 64MB
            block_size: 64 * 1024, // 64KB
            use_bloom_filters: true,
            bloom_bits_per_key: 10.0,
            use_ribbon_filters: false,
            compression: CompressionAlgorithm::Lz4,
        }
    }
}

/// Compaction configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompactionConfig {
    /// Compaction strategy
    pub strategy: CompactionStrategy,
    /// Maximum number of background threads
    pub max_threads: usize,
    /// I/O rate limit in MB/s
    pub io_rate_limit: Option<u64>,
    /// Whether to use RL-driven compaction
    pub use_rl_agent: bool,
    /// Compaction trigger thresholds
    pub triggers: CompactionTriggers,
}

impl Default for CompactionConfig {
    fn default() -> Self {
        Self {
            strategy: CompactionStrategy::Leveled,
            max_threads: 4,
            io_rate_limit: Some(100), // 100 MB/s
            use_rl_agent: true,
            triggers: CompactionTriggers::default(),
        }
    }
}

/// Compaction strategy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CompactionStrategy {
    /// Leveled compaction (RocksDB-style)
    Leveled,
    /// Tiered compaction
    Tiered,
    /// Flexible LSM (FLSM) - can switch between strategies
    Flexible,
}

/// Compaction trigger thresholds
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompactionTriggers {
    /// Level 0 file count threshold
    pub level0_files: usize,
    /// Level size ratio threshold
    pub level_size_ratio: f64,
    /// Write amplification threshold
    pub write_amplification: f64,
}

impl Default for CompactionTriggers {
    fn default() -> Self {
        Self {
            level0_files: 4,
            level_size_ratio: 10.0,
            write_amplification: 5.0,
        }
    }
}

/// Cache configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    /// Block cache size in bytes
    pub block_cache_size: usize,
    /// Value log cache size in bytes
    pub vlog_cache_size: usize,
    /// Cache eviction policy
    pub eviction_policy: EvictionPolicy,
    /// Whether to use unified cache
    pub unified_cache: bool,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            block_cache_size: 256 * 1024 * 1024, // 256MB
            vlog_cache_size: 64 * 1024 * 1024, // 64MB
            eviction_policy: EvictionPolicy::Arc,
            unified_cache: true,
        }
    }
}

/// Cache eviction policy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EvictionPolicy {
    /// LRU (Least Recently Used)
    Lru,
    /// ARC (Adaptive Replacement Cache)
    Arc,
    /// TinyLFU
    TinyLfu,
}

/// Learned index configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearnedIndexConfig {
    /// Whether to enable learned indexes
    pub enabled: bool,
    /// Model type to use
    pub model_type: ModelType,
    /// Training frequency (every N operations)
    pub training_frequency: usize,
    /// Whether to use online tuning
    pub online_tuning: bool,
    /// Fallback search method
    pub fallback_method: FallbackMethod,
}

impl Default for LearnedIndexConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            model_type: ModelType::PiecewiseLinear,
            training_frequency: 10000,
            online_tuning: true,
            fallback_method: FallbackMethod::BinarySearch,
        }
    }
}

/// Learned index model type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ModelType {
    /// Piecewise linear regression
    PiecewiseLinear,
    /// Recursive Model Index (RMI)
    Rmi,
    /// Tiny neural network
    TinyNn,
}

/// Fallback search method when learned index fails
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FallbackMethod {
    /// Binary search
    BinarySearch,
    /// Fence pointers
    FencePointers,
    /// Bloom filter + scan
    BloomScan,
}

/// RL agent configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RlAgentConfig {
    /// Whether to enable RL agent
    pub enabled: bool,
    /// Learning rate
    pub learning_rate: f64,
    /// Exploration rate (epsilon)
    pub exploration_rate: f64,
    /// State update frequency
    pub state_update_frequency: usize,
    /// Whether to use offline training
    pub offline_training: bool,
    /// Training data path
    pub training_data_path: Option<PathBuf>,
}

impl Default for RlAgentConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            learning_rate: 0.01,
            exploration_rate: 0.1,
            state_update_frequency: 1000,
            offline_training: false,
            training_data_path: None,
        }
    }
}

/// Performance tuning configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceConfig {
    /// Number of worker threads
    pub worker_threads: usize,
    /// I/O buffer size
    pub io_buffer_size: usize,
    /// Whether to use direct I/O
    pub direct_io: bool,
    /// Whether to use memory-mapped files
    pub memory_mapped: bool,
    /// NUMA awareness
    pub numa_aware: bool,
}

impl Default for PerformanceConfig {
    fn default() -> Self {
        Self {
            worker_threads: num_cpus::get(),
            io_buffer_size: 1024 * 1024, // 1MB
            direct_io: false,
            memory_mapped: true,
            numa_aware: false,
        }
    }
}

impl Config {
    /// Create a new configuration with default values
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the database path
    pub fn with_db_path(mut self, path: PathBuf) -> Self {
        self.db_path = path;
        self
    }

    /// Set WAL configuration
    pub fn with_wal(mut self, wal: WalConfig) -> Self {
        self.wal = wal;
        self
    }

    /// Set value log configuration
    pub fn with_value_log(mut self, vlog: ValueLogConfig) -> Self {
        self.value_log = vlog;
        self
    }

    /// Set memtable configuration
    pub fn with_memtable(mut self, memtable: MemtableConfig) -> Self {
        self.memtable = memtable;
        self
    }

    /// Set SST configuration
    pub fn with_sst(mut self, sst: SstConfig) -> Self {
        self.sst = sst;
        self
    }

    /// Set compaction configuration
    pub fn with_compaction(mut self, compaction: CompactionConfig) -> Self {
        self.compaction = compaction;
        self
    }

    /// Set cache configuration
    pub fn with_cache(mut self, cache: CacheConfig) -> Self {
        self.cache = cache;
        self
    }

    /// Set learned index configuration
    pub fn with_learned_index(mut self, learned_index: LearnedIndexConfig) -> Self {
        self.learned_index = learned_index;
        self
    }

    /// Set RL agent configuration
    pub fn with_rl_agent(mut self, rl_agent: RlAgentConfig) -> Self {
        self.rl_agent = rl_agent;
        self
    }

    /// Set performance configuration
    pub fn with_performance(mut self, performance: PerformanceConfig) -> Self {
        self.performance = performance;
        self
    }

    /// Validate the configuration
    pub fn validate(&self) -> Result<(), String> {
        if self.wal.max_file_size == 0 {
            return Err("WAL max file size must be greater than 0".to_string());
        }
        if self.value_log.max_segment_size == 0 {
            return Err("Value log max segment size must be greater than 0".to_string());
        }
        if self.memtable.max_size == 0 {
            return Err("Memtable max size must be greater than 0".to_string());
        }
        if self.sst.target_file_size == 0 {
            return Err("SST target file size must be greater than 0".to_string());
        }
        if self.cache.block_cache_size == 0 {
            return Err("Block cache size must be greater than 0".to_string());
        }
        Ok(())
    }
}
