use thiserror::Error;

/// Result type for AuraDB operations
pub type Result<T> = std::result::Result<T, Error>;

/// Error types for AuraDB operations
#[derive(Error, Debug)]
pub enum Error {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Bincode error: {0}")]
    Bincode(#[from] bincode::Error),

    #[error("Key not found: {0}")]
    KeyNotFound(String),

    #[error("Invalid value pointer: {0}")]
    InvalidValuePointer(String),

    #[error("WAL corruption: {0}")]
    WalCorruption(String),

    #[error("SST corruption: {0}")]
    SstCorruption(String),

    #[error("Value log corruption: {0}")]
    ValueLogCorruption(String),

    #[error("Compaction error: {0}")]
    Compaction(String),

    #[error("Cache error: {0}")]
    Cache(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Learned index error: {0}")]
    LearnedIndex(String),

    #[error("RL agent error: {0}")]
    RlAgent(String),

    #[error("Memory allocation error: {0}")]
    Memory(String),

    #[error("Concurrency error: {0}")]
    Concurrency(String),

    #[error("Unknown error: {0}")]
    Unknown(String),
}

impl From<&str> for Error {
    fn from(s: &str) -> Self {
        Error::Unknown(s.to_string())
    }
}

impl From<String> for Error {
    fn from(s: String) -> Self {
        Error::Unknown(s)
    }
}
