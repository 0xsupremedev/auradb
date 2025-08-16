# AuraDB 🚀

A high-performance Rust storage engine that combines **WAL-time Key-Value separation**, **RL-driven adaptive compaction**, and **learned indexes** to rival and surpass RocksDB performance.

## 🎯 Key Innovations

### 1. WAL-time Key-Value Separation (BVLSM-inspired)
- **Why**: Classic LSM engines copy large values repeatedly during flush/compaction, causing high write amplification
- **How**: Large values are immediately written to a separate value log at WAL time, while only keys and pointers go to the LSM tree
- **Benefits**: 7.6× higher throughput on 64KB random writes vs RocksDB, reduced memory pressure, stable I/O patterns

### 2. RL-driven Adaptive Compaction (RusKey-inspired)
- **Why**: Static compaction policies struggle with dynamic workloads
- **How**: Reinforcement learning agent continuously tunes LSM structure between tiered/leveled modes
- **Benefits**: Up to 4× better end-to-end performance under changing workloads, reduced tail latency

### 3. Learned Indexes (DobLIX-inspired)
- **Why**: Traditional indexes can be slow and memory-intensive
- **How**: Machine learning models predict data location with fallback to binary search
- **Benefits**: 1.19×–2.21× throughput improvement, 70% faster than cache-optimized B-trees

## 🏗️ Architecture

```
Client API (KV + optional SQL-ish ops)
  └── Router (point/scan/batch/txn)
      ├── Txn/TSO (optional MVCC)
      ├── Read Path
      │    ├── Learned Index Tier (+ fallback)
      │    ├── Block Cache (+ Bloom/Ribbon filters)
      │    └── SST Manager (point/range reads)
      └── Write Path
           ├── WAL-time KV Separation (KV router)
           │    ├── WAL (keys + meta only)
           │    └── Value Log (separate big values)
           ├── Memtable(s) (skiplist/ART)
           └── Flush & SST Builder

Background services
  ├── RL Compaction Orchestrator (policy + scheduler)
  ├── GC for Value Log (live-pointer tracing)
  ├── Learned Index Trainer/Tuner (online/offline)
  ├── IO Scheduler (rate limit, debt accounting)
  └── Telemetry + Self-tuning (A/B configs)

Storage
  ├── SST files (tiered LSM)
  ├── Value log segments
  └── Manifests + snapshots
```

## 🚀 Quick Start

### Prerequisites
- Rust 1.70+ 
- Linux/macOS/Windows

### Installation

```bash
# Clone the repository
git clone https://github.com/auradb/auradb.git
cd auradb

# Build the project
cargo build --release

# Run tests
cargo test

# Run the example
cargo run --example basic_usage
```

### Basic Usage

```rust
use auradb::prelude::*;

#[tokio::main]
async fn main() -> Result<()> {
    // Create engine
    let engine = EngineBuilder::new()
        .with_db_path("/path/to/db".into())
        .build()
        .await?;
    
    // Basic operations
    engine.put_str("key1", "value1").await?;
    let value = engine.get_str("key1").await?;
    println!("Value: {:?}", value);
    
    // Range scan
    let results = engine.scan_str("a", "z", Some(100)).await?;
    
    // Clean up
    engine.close().await?;
    Ok(())
}
```

## 📊 Performance Targets

| Metric | Target | vs RocksDB |
|--------|--------|-------------|
| Large-value writes | ≥2× throughput | BVLSM effect |
| Dynamic workloads | ≥1.5–3× p99 stability | RL compaction |
| Point reads | ~1.2–2× throughput | Learned indexes |
| Memory usage | Equal or lower | Efficient structures |

## 🛠️ Configuration

### Basic Configuration

```rust
use auradb::config::Config;

let config = Config::new()
    .with_db_path("/path/to/db".into())
    .with_wal(WalConfig {
        max_file_size: 64 * 1024 * 1024, // 64MB
        async_writes: true,
        ..Default::default()
    })
    .with_value_log(ValueLogConfig {
        separation_threshold: 1024, // 1KB
        max_segment_size: 256 * 1024 * 1024, // 256MB
        ..Default::default()
    });

let engine = create_engine_with_config(config).await?;
```

### Advanced Configuration

```rust
let engine = AdvancedEngineBuilder::new()
    .with_db_path("/path/to/db".into())
    .with_memtable_config(MemtableConfig {
        implementation: MemtableImpl::SkipList,
        max_size: 128 * 1024 * 1024, // 128MB
        ..Default::default()
    })
    .with_learned_index_config(LearnedIndexConfig {
        model_type: ModelType::PiecewiseLinear,
        online_tuning: true,
        ..Default::default()
    })
    .with_rl_agent_config(RlAgentConfig {
        learning_rate: 0.01,
        exploration_rate: 0.1,
        ..Default::default()
    })
    .build()
    .await?;
```

## 🔧 Core Components

### 1. WAL (Write-Ahead Log)
- Append-only log with configurable sync policies
- Async writes for high throughput
- Automatic file rotation

### 2. Value Log
- Parallel write queues for high throughput
- Compression support (LZ4, Zstd, Snappy)
- Automatic segment rotation

### 3. Memtable
- Multiple implementations: SkipList, ART, B-tree
- Lock-free operations where possible
- Automatic flushing based on size thresholds

### 4. SST Manager
- Multi-level LSM structure
- Configurable block sizes and compression
- Bloom/Ribbon filters for fast lookups

### 5. Compaction
- Flexible LSM (FLSM) supporting tiered/leveled modes
- RL-driven policy selection
- I/O rate limiting and debt accounting

### 6. Learned Indexes
- Piecewise linear regression models
- Online tuning and validation
- Fallback to traditional search methods

## 📈 Roadmap

### M0 – Core Skeleton (2–3 weeks) ✅
- [x] Basic WAL and memtable
- [x] Simple flush to SST (no compaction)
- [x] Basic block cache and Bloom filters

### M1 – WAL-time KV Separation (BVLSM-lite) ✅
- [x] Value log implementation
- [x] WAL-time separation logic
- [x] Async value log writes

### M2 – Basic Compaction (Tiered+Leveled) 🚧
- [ ] Tiered and leveled compaction
- [ ] I/O budgeting and admission control
- [ ] Manual policy switching

### M3 – RL Agent (RusKey-style) 📋
- [ ] RL agent implementation
- [ ] State observation and action selection
- [ ] Safety fallbacks and rollbacks

### M4 – Learned Indexes (DobLIX-style) 📋
- [ ] Piecewise linear models
- [ ] Online tuner and validation
- [ ] Fallback search methods

### M5 – Production Features 📋
- [ ] Value log GC
- [ ] Crash recovery and snapshots
- [ ] Backup and restore

### M6 – Optimization 📋
- [ ] NUMA-aware threading
- [ ] Advanced QoS and admission control
- [ ] Encryption and advanced compression

## 🧪 Testing & Benchmarks

### Running Tests
```bash
# Unit tests
cargo test

# Integration tests
cargo test --test integration

# Benchmarks
cargo bench
```

### Benchmarking
```bash
# YCSB-style workloads
cargo run --bin benchmark -- --workload ycsb

# Custom traces
cargo run --bin benchmark -- --trace-file workload.trace

# Comparison with RocksDB
cargo run --bin benchmark -- --compare-rocksdb
```

## 🤝 Contributing

We welcome contributions! Please see our [Contributing Guide](CONTRIBUTING.md) for details.

### Development Setup
```bash
# Clone and setup
git clone https://github.com/auradb/auradb.git
cd auradb

# Install development dependencies
cargo install cargo-watch
cargo install cargo-audit

# Run development server
cargo watch -x check -x test -x run
```

### Code Style
- Follow Rust formatting guidelines (`cargo fmt`)
- Run clippy (`cargo clippy`)
- Ensure all tests pass
- Add tests for new functionality

## 📚 Documentation

- [API Reference](https://docs.rs/auradb)
- [Architecture Guide](docs/architecture.md)
- [Performance Tuning](docs/performance.md)
- [Troubleshooting](docs/troubleshooting.md)

## 🔬 Research & References

This project builds on several key research papers:

- **BVLSM**: WAL-time Key-Value separation for LSM engines
- **RusKey**: RL-driven compaction for dynamic workloads  
- **DobLIX**: Dual-objective learned indexes for LSM engines
- **Learned Indexes**: Machine learning for data structures

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
