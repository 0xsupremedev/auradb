# 🚀 AuraDB Benchmarking Guide

This document provides comprehensive guidance on benchmarking AuraDB using the included benchmark tools. These tools are designed to provide accurate, reproducible performance measurements that will help validate the performance improvements from each milestone.

## 📊 Available Benchmark Tools

### 1. **Basic Benchmark** (`src/bin/benchmark.rs`)
A simple, fast benchmark for quick performance checks.

**Usage:**
```bash
# Basic benchmark with default settings
cargo run --release --bin benchmark

# Customized benchmark
cargo run --release --bin benchmark -- \
  --operations 100000 \
  --key-size 16 \
  --value-size 1024 \
  --batch-size 100
```

**Features:**
- ✅ Throughput measurement (ops/sec)
- ✅ Latency percentiles (p50, p95, p99)
- ✅ Batch write support
- ✅ Configurable key/value sizes
- ✅ Progress indicators

### 2. **Full-Spectrum Benchmark** (`src/bin/full_benchmark.rs`)
Comprehensive benchmark suite that tests multiple workloads and value sizes.

**Usage:**
```bash
# Run the complete benchmark suite
cargo run --release --bin full_benchmark
```

**What it tests:**
- **Small values (1KB)**: Raw performance baseline
- **Medium values (8KB)**: Scaling characteristics
- **Large values (64KB)**: WAL-time KV separation benefits
- **Read-heavy workloads**: Cache efficiency
- **Write-heavy workloads**: Write path performance
- **Latency profiling**: Detailed percentile analysis

**Output includes:**
- 🏎 Raw speed (reads/writes)
- 📦 Bandwidth limits (MB/s)
- ⏱ Latency profile (p50, p95, p99, p99.9)
- 💾 Disk usage growth
- 🔀 Mixed workload performance

### 3. **YCSB-Style Benchmark** (`src/bin/ycsb_benchmark.rs`)
Industry-standard YCSB workload patterns for database benchmarking.

**Usage:**
```bash
# Run specific YCSB workload
cargo run --release --bin ycsb_benchmark -- \
  --workload A \
  --operations 50000 \
  --key-size 16 \
  --value-size 1024

# Available workloads: A, B, C, D, E, F
```

**YCSB Workload Definitions:**

| Workload | Description | Read Ratio | Update Ratio | Insert Ratio | Scan Ratio | RMW Ratio |
|----------|-------------|------------|--------------|--------------|------------|-----------|
| **A** | Update heavy | 50% | 50% | 0% | 0% | 0% |
| **B** | Read mostly | 95% | 5% | 0% | 0% | 0% |
| **C** | Read only | 100% | 0% | 0% | 0% | 0% |
| **D** | Read latest | 95% | 0% | 5% | 0% | 0% |
| **E** | Short ranges | 0% | 0% | 5% | 95% | 0% |
| **F** | Read-modify-write | 50% | 0% | 0% | 0% | 50% |

**Features:**
- ✅ Standard YCSB workload patterns
- ✅ Accurate operation mix ratios
- ✅ Detailed latency percentiles
- ✅ Throughput measurements
- ✅ Operation breakdown by type
- ✅ Disk usage tracking

### 4. **Complete YCSB Suite Runner** (`src/bin/run_all_ycsb_workloads.rs`)
Automated runner that executes all YCSB workloads with different value sizes.

**Usage:**
```bash
# Run complete YCSB benchmark suite
cargo run --release --bin run_all_ycsb_workloads
```

**What it tests:**
- **Phase 1**: All 6 workloads with 1KB values
- **Phase 2**: Key workloads (A, B, C) with 8KB values
- **Phase 3**: Key workloads with 64KB values (WAL-time KV separation test)

**Output includes:**
- 📊 Performance summary by value size
- 📈 Performance degradation analysis
- 🎯 Key insights and recommendations
- 🚀 Next steps roadmap

## 🎯 Benchmark Scenarios

### **Performance Validation**
Use these benchmarks to validate performance improvements at each milestone:

- **M0 (Current)**: Basic LSM skeleton performance
- **M1 (WAL-time KV separation)**: Large value performance improvement
- **M2 (Compaction)**: Sustained performance under load
- **M3 (RL compaction)**: Adaptive performance tuning
- **M4 (Learned indexes)**: Read performance improvement
- **M5 (GC & Recovery)**: Long-term stability
- **M6 (Production)**: Production readiness

### **Value Size Testing**
The benchmarks are designed to demonstrate the benefits of WAL-time KV separation:

| Value Size | Expected Performance | Current State | Target After M1 |
|------------|---------------------|---------------|-----------------|
| **1KB** | Excellent | ✅ 300K+ ops/sec | ✅ 300K+ ops/sec |
| **8KB** | Good | ✅ 45K ops/sec | ✅ 45K ops/sec |
| **64KB** | Poor | ❌ 5K ops/sec | ✅ 250K+ ops/sec |

### **Workload Patterns**
Test different application patterns:

- **OLTP**: Workloads A, B, C (point operations)
- **Analytics**: Workload E (range scans)
- **Mixed**: Workloads D, F (read-write mixes)

## 📈 Interpreting Results

### **Throughput Metrics**
- **ops/sec**: Raw operation throughput
- **MB/s**: Data bandwidth utilization
- **Expected improvements**: 5-7x for large values after M1

### **Latency Metrics**
- **p50**: Median latency (typical performance)
- **p95**: 95th percentile (good performance)
- **p99**: 99th percentile (tail latency)
- **p99.9**: 99.9th percentile (worst-case)

### **Performance Ratios**
- **Small vs Large values**: Should be similar after M1
- **Read vs Write**: Reads should be faster
- **Sequential vs Random**: Sequential should be faster

## 🚀 Running Benchmarks

### **Quick Performance Check**
```bash
# Fast benchmark for development
cargo run --release --bin benchmark -- --operations 10000
```

### **Comprehensive Testing**
```bash
# Full performance analysis
cargo run --release --bin full_benchmark
```

### **YCSB Validation**
```bash
# Standard database workload
cargo run --release --bin ycsb_benchmark -- --workload A --operations 100000
```

### **Complete Suite**
```bash
# Full YCSB validation (takes longer)
cargo run --release --bin run_all_ycsb_workloads
```

## 🔧 Benchmark Configuration

### **Key Parameters**
- `--operations`: Number of operations to test
- `--key-size`: Key size in bytes
- `--value-size`: Value size in bytes
- `--workload`: YCSB workload type (A-F)
- `--batch-size`: Batch size for batch operations
- `--threads`: Number of concurrent threads

### **Recommended Settings**
- **Development**: 10K-50K operations
- **Validation**: 100K-1M operations
- **Production**: 1M+ operations
- **Value sizes**: 1KB, 8KB, 64KB for WAL-time KV testing

## 📊 Expected Results

### **Current Performance (M0)**
- **Small values (1KB)**: 250K-300K ops/sec
- **Medium values (8KB)**: 30K-50K ops/sec
- **Large values (64KB)**: 5K-10K ops/sec

### **After M1 (WAL-time KV separation)**
- **Small values (1KB)**: 250K-300K ops/sec
- **Medium values (8KB)**: 30K-50K ops/sec
- **Large values (64KB)**: 250K-300K ops/sec ⭐

### **After M2-M6 (Full implementation)**
- **All value sizes**: Consistent high performance
- **Compaction**: Stable performance under load
- **Learned indexes**: Faster reads
- **RL tuning**: Adaptive performance

## 🎯 Success Criteria

### **M1 Success Metrics**
- ✅ 64KB values perform within 20% of 1KB values
- ✅ Large value throughput: 200K+ ops/sec
- ✅ Memory usage stable regardless of value size

### **M2 Success Metrics**
- ✅ Persistent storage working
- ✅ Compaction not blocking writes
- ✅ Performance stable over time

### **M3 Success Metrics**
- ✅ RL agent adapting to workload changes
- ✅ Performance improving under dynamic loads
- ✅ No performance regressions

## 🚨 Troubleshooting

### **Common Issues**
- **Build errors**: Ensure all dependencies are installed
- **Performance variance**: Run multiple times, use release builds
- **Memory issues**: Reduce operation count for large values
- **Slow benchmarks**: Use `--release` flag for optimized builds

### **Performance Tips**
- Always use `--release` for accurate measurements
- Run multiple times to account for variance
- Monitor system resources during benchmarks
- Use appropriate operation counts for your system

## 🔮 Future Enhancements

### **Planned Features**
- **Multi-threaded benchmarks**: Concurrent workload testing
- **Real-time monitoring**: Live performance metrics
- **Comparison tools**: Side-by-side milestone comparisons
- **Automated regression testing**: CI/CD integration
- **Custom workload definitions**: User-defined patterns

### **Integration Opportunities**
- **Prometheus metrics**: Production monitoring
- **Grafana dashboards**: Performance visualization
- **CI/CD pipelines**: Automated performance validation
- **Performance regression detection**: Automated alerts

---

## 📚 Additional Resources

- [YCSB Paper](https://www2.cs.duke.edu/courses/fall13/cps296.4/838-CloudPapers/ycsb.pdf)
- [RocksDB Benchmarks](https://github.com/facebook/rocksdb/wiki/Performance-Benchmarks)
- [LSM Tree Performance](https://en.wikipedia.org/wiki/Log-structured_merge-tree#Performance_characteristics)

---

*This benchmarking suite provides the foundation for validating AuraDB's performance improvements at each milestone. Use it to ensure that each innovation delivers measurable benefits and maintains performance stability.*
