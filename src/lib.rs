//! AuraDB - High-performance Rust storage engine
//! 
//! This crate provides a storage engine with the following key innovations:
//! - WAL-time Key-Value separation (BVLSM-inspired)
//! - Adaptive (RL-driven) compaction (RusKey-inspired) 
//! - Learned indexes (DobLIX-inspired)
//! 
//! # Quick Start
//! 
//! ```rust
//! use auradb::{Engine, EngineBuilder};
//! 
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let engine = EngineBuilder::new()
//!         .path("./auradb_data")
//!         .build()?;
//!     
//!     engine.put_str("key", "value")?;
//!     let value = engine.get_str("key")?;
//!     println!("Value: {:?}", value);
//!     
//!     engine.close().await?;
//!     Ok(())
//! }
//! ```

pub mod error;
pub mod storage;
pub mod config;
pub mod api;

// Re-export main types
pub use api::{Engine, EngineBuilder, AuraEngine};
pub use storage::{Key, Value, ValuePointer, Entry, Batch, Range};
pub use error::{Error, Result};

/// Common imports for the crate
pub mod prelude {
    pub use crate::{Engine, EngineBuilder, AuraEngine};
    pub use crate::{Key, Value, ValuePointer, Entry, Batch, Range};
    pub use crate::{Error, Result};
}
