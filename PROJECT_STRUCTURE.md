# AuraDB Project Structure

This document provides an overview of the AuraDB project structure and organization.

## 📁 Directory Structure

```
auradb/
├── src/                    # Source code
│   ├── lib.rs             # Main library entry point
│   ├── api.rs             # Public API and engine interface
│   ├── engine.rs          # Main engine implementation
│   ├── config.rs          # Configuration structures
│   ├── error.rs           # Error types and handling
│   ├── storage.rs         # Core storage types (Key, Value, etc.)
│   ├── wal.rs             # Write-Ahead Log implementation
│   ├── vlog.rs            # Value Log for WAL-time KV separation
│   ├── memtable.rs        # In-memory table implementations
│   ├── sst.rs             # SSTable management (placeholder)
│   ├── compactor.rs       # Compaction logic (placeholder)
│   ├── index.rs           # Learned index implementation (placeholder)
│   ├── cache.rs           # Block and value log caching (placeholder)
│   ├── gc.rs              # Garbage collection (placeholder)
│   ├── telemetry.rs       # Metrics and monitoring (placeholder)
│   └── metrics.rs         # Metrics collection (placeholder)
├── examples/               # Usage examples
│   └── basic_usage.rs     # Basic usage demonstration
├── src/bin/               # Binary executables
│   └── benchmark.rs       # Benchmarking tool
├── tests/                 # Integration tests (placeholder)
├── benches/               # Performance benchmarks (placeholder)
├── docs/                  # Documentation (placeholder)
├── Cargo.toml            # Project dependencies and metadata
├── Makefile              # Common development tasks
├── README.md             # Project overview and quick start
└── PROJECT_STRUCTURE.md  # This file
```

## 🏗️ Architecture Overview

### Core Components (Implemented)

#### 1. **API Layer** (`src/api.rs`)
- **Purpose**: Public interface for the storage engine
- **Key Features**:
  - `Engine` trait defining core operations
  - `EngineBuilder` for configuration
  - `AuraEngine` main implementation
  - Convenience methods for common operations

#### 2. **Configuration** (`src/config.rs`)
- **Purpose**: Comprehensive configuration management
- **Key Features**:
  - WAL configuration (sync policies, file sizes)
  - Value log configuration (separation thresholds, compression)
  - Memtable configuration (size limits, implementations)
  - Compaction configuration (strategies, triggers)
  - Cache configuration (eviction policies, sizes)
  - Learned index configuration (model types, tuning)
  - RL agent configuration (learning rates, exploration)

#### 3. **Storage Types** (`src/storage.rs`)
- **Purpose**: Core data structures and types
- **Key Features**:
  - `Key` and `Value` structures
  - `ValuePointer` for WAL-time KV separation
  - `Entry` for key-value operations
  - `Batch` for bulk operations
  - `Range` for scan operations

#### 4. **WAL** (`src/wal.rs`)
- **Purpose**: Write-Ahead Logging for durability
- **Key Features**:
  - Append-only log with configurable sync policies
  - Async writes for high throughput
  - Automatic file rotation
  - Support for different record types
  - Recovery and replay capabilities

#### 5. **Value Log** (`src/vlog.rs`)
- **Purpose**: WAL-time Key-Value separation
- **Key Features**:
  - Parallel write queues for high throughput
  - Compression support (LZ4, Zstd, Snappy)
  - Automatic segment rotation
  - Checksum validation
  - Efficient value retrieval

#### 6. **Memtable** (`src/memtable.rs`)
- **Purpose**: In-memory sorted table for fast writes
- **Key Features**:
  - Multiple implementations (SkipList, ART, B-tree)
  - Lock-free operations where possible
  - Memory usage tracking
  - Automatic flushing based on size thresholds

#### 7. **Error Handling** (`src/error.rs`)
- **Purpose**: Comprehensive error types and handling
- **Key Features**:
  - Custom error types for different components
  - Conversion from standard library errors
  - Descriptive error messages
  - Result type aliases

### Core Components (Planned)

#### 8. **SST Manager** (`src/sst.rs`)
- **Purpose**: Sorted String Table management
- **Planned Features**:
  - Multi-level LSM structure
  - Block-based storage with compression
  - Bloom/Ribbon filters
  - Index management

#### 9. **Compactor** (`src/compactor.rs`)
- **Purpose**: LSM compaction orchestration
- **Planned Features**:
  - Flexible LSM (FLSM) implementation
  - Tiered and leveled compaction
  - RL-driven policy selection
  - I/O rate limiting and debt accounting

#### 10. **Learned Indexes** (`src/index.rs`)
- **Purpose**: Machine learning-based indexing
- **Planned Features**:
  - Piecewise linear regression models
  - Online tuning and validation
  - Fallback to traditional search methods
  - Model registry and management

#### 11. **Cache** (`src/cache.rs`)
- **Purpose**: Block and value log caching
- **Planned Features**:
  - Unified cache for SST blocks and vlog pages
  - Multiple eviction policies (ARC, TinyLFU)
  - Admission control and size management

#### 12. **Garbage Collection** (`src/gc.rs`)
- **Purpose**: Value log garbage collection
- **Planned Features**:
  - Live pointer tracing from SSTs
  - Incremental reclamation
  - Background GC with I/O credits

#### 13. **Telemetry** (`src/telemetry.rs`)
- **Purpose**: Metrics and monitoring
- **Planned Features**:
  - Performance metrics collection
  - Health monitoring
  - A/B configuration testing
  - Self-tuning capabilities

## 🔄 Data Flow

### Write Path
1. **Client Request** → `Engine::put()`
2. **WAL-time KV Separation** → Check value size threshold
3. **Large Values** → Write to value log, get pointer
4. **WAL Write** → Append to WAL (key + pointer/metadata)
5. **Memtable Insert** → Add to in-memory table
6. **Background Flush** → Write to SST when threshold reached

### Read Path
1. **Client Request** → `Engine::get()`
2. **Memtable Check** → Search in-memory table first
3. **Value Resolution** → If pointer, read from value log
4. **SST Search** → Search on-disk tables (planned)
5. **Learned Index** → Use ML models for fast lookup (planned)
6. **Fallback** → Traditional search methods if needed

## 🧪 Testing Strategy

### Unit Tests
- **Location**: Each module contains its own tests
- **Coverage**: Core functionality and edge cases
- **Execution**: `cargo test`

### Integration Tests
- **Location**: `tests/` directory (planned)
- **Coverage**: End-to-end functionality
- **Execution**: `cargo test --test integration`

### Benchmarks
- **Location**: `benches/` directory (planned)
- **Coverage**: Performance characteristics
- **Execution**: `cargo bench`

### Example Programs
- **Location**: `examples/` directory
- **Purpose**: Demonstrate usage patterns
- **Execution**: `cargo run --example basic_usage`

## 🚀 Development Workflow

### 1. **Setup Development Environment**
```bash
make setup          # Install development tools
make install        # Install additional dependencies
```

### 2. **Daily Development Cycle**
```bash
make dev            # Format, lint, and test
make watch          # Watch for changes and run tests
```

### 3. **Quality Assurance**
```bash
make ci             # Run all checks (format, lint, test)
make full-dev       # Full development cycle including benchmarks
```

### 4. **Performance Testing**
```bash
make benchmark      # Run basic benchmarks
make benchmark-large # Test WAL-time KV separation
make profile        # Performance profiling
```

## 📊 Code Organization Principles

### 1. **Separation of Concerns**
- Each module has a single, well-defined responsibility
- Clear interfaces between components
- Minimal coupling between modules

### 2. **Async-First Design**
- All I/O operations are asynchronous
- Tokio runtime for high-performance async execution
- Non-blocking operations throughout the stack

### 3. **Configuration-Driven**
- Comprehensive configuration options
- Sensible defaults for common use cases
- Runtime configuration changes where possible

### 4. **Error Handling**
- Custom error types for different failure modes
- Comprehensive error context and messages
- Graceful degradation where possible

### 5. **Performance Focus**
- Zero-copy operations where possible
- Efficient memory management
- Lock-free data structures where applicable

## 🔮 Future Extensions

### 1. **SQL Interface**
- SQL parser and planner
- Query optimization
- Transaction support

### 2. **Distributed Features**
- Replication and consistency
- Sharding and partitioning
- Cluster management

### 3. **Advanced Analytics**
- Columnar storage
- Aggregation and analytics
- Machine learning pipelines

### 4. **Cloud Integration**
- Cloud storage backends
- Auto-scaling and elasticity
- Multi-region deployment

## 📝 Contributing Guidelines

### 1. **Code Style**
- Follow Rust formatting guidelines
- Use meaningful variable and function names
- Add comprehensive documentation

### 2. **Testing**
- Write tests for new functionality
- Ensure all tests pass before submitting
- Add integration tests for complex features

### 3. **Documentation**
- Update relevant documentation
- Add examples for new features
- Include performance characteristics

### 4. **Performance**
- Benchmark new features
- Optimize critical paths
- Monitor memory usage and I/O patterns

---

This structure provides a solid foundation for building a high-performance storage engine while maintaining code quality and developer productivity.
