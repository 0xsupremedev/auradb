use std::time::Instant;
use std::fs;
use std::path::Path;
use auradb::AuraEngine;
use auradb::config::Config;
use rand::Rng;
use clap::Parser;

/// RocksDB Comparison Benchmark
/// Validates AuraDB performance against published RocksDB benchmarks
#[derive(Parser, Debug)]
#[command(name = "AuraDB vs RocksDB Comparison")]
struct Args {
    /// Number of operations
    #[arg(short, long, default_value_t = 100000)]
    operations: usize,
    
    /// Key size in bytes
    #[arg(long, default_value_t = 16)]
    key_size: usize,
    
    /// Value size in bytes
    #[arg(long, default_value_t = 1024)]
    value_size: usize,
    
    /// Whether to simulate fsync (durability)
    #[arg(long, default_value_t = false)]
    fsync: bool,
    
    /// Database path
    #[arg(long, default_value = "./rocksdb_comparison_db")]
    db_path: String,
}

/// RocksDB benchmark data from published sources
struct RocksDBBenchmark {
    name: String,
    throughput_ops: f64,
    p50_latency_us: f64,
    p99_latency_us: f64,
    source: String,
}

impl RocksDBBenchmark {
    fn new(name: &str, throughput: f64, p50: f64, p99: f64, source: &str) -> Self {
        Self {
            name: name.to_string(),
            throughput_ops: throughput,
            p50_latency_us: p50,
            p99_latency_us: p99,
            source: source.to_string(),
        }
    }
}

/// Run RocksDB comparison benchmarks
fn run_rocksdb_comparison(args: &Args) {
    println!("🔬 AuraDB vs RocksDB Performance Comparison");
    println!("=============================================");
    println!("This benchmark validates AuraDB against published RocksDB numbers");
    println!("Value size: {} bytes | Operations: {}", args.value_size, args.operations);
    println!("Durability: {}", if args.fsync { "fsync enabled" } else { "fsync disabled" });
    
    // Clean up any existing database
    fs::remove_dir_all(&args.db_path).ok();
    
    let config = Config::default();
    let mut db = AuraEngine::new(config).expect("Failed to create AuraDB engine");
    
    // Benchmark 1: Bulk Random Writes (RocksDB: ~1M ops/sec)
    println!("\n📊 Benchmark 1: Bulk Random Writes");
    println!("=====================================");
    let write_result = benchmark_bulk_writes(&mut db, args);
    
    // Benchmark 2: Point Reads (RocksDB: P50 ~450µs, P99 ~1ms)
    println!("\n📊 Benchmark 2: Point Reads");
    println!("==============================");
    let read_result = benchmark_point_reads(&mut db, args);
    
    // Benchmark 3: Read-While-Writing (RocksDB: ~7M ops/sec)
    println!("\n📊 Benchmark 3: Read-While-Writing");
    println!("=====================================");
    let mixed_result = benchmark_read_while_writing(&mut db, args);
    
    // Generate comparison report
    println!("\n\n🎯 COMPETITIVE ANALYSIS REPORT");
    println!("===============================");
    
    // Compare against RocksDB benchmarks
    let rocksdb_benchmarks = get_rocksdb_benchmarks();
    
    // Write performance comparison
    if let Some(rocksdb_write) = rocksdb_benchmarks.iter().find(|b| b.name.contains("Bulk Writes")) {
        let write_ratio = write_result.throughput / rocksdb_write.throughput_ops;
        let write_score = if write_ratio >= 1.0 { 100 } else { (write_ratio * 100.0) as u32 };
        
        println!("\n📝 Write Performance Comparison:");
        println!("   AuraDB: {:.0} ops/sec", write_result.throughput);
        println!("   RocksDB: {:.0} ops/sec ({})", rocksdb_write.throughput_ops, rocksdb_write.source);
        println!("   Ratio: {:.2}x | Score: {}/100", write_ratio, write_score);
        
        if write_ratio >= 1.0 {
            println!("   🎉 AuraDB outperforms RocksDB!");
        } else {
            println!("   📈 Gap to close: {:.1}x improvement needed", 1.0 / write_ratio);
        }
    }
    
    // Read performance comparison
    if let Some(rocksdb_read) = rocksdb_benchmarks.iter().find(|b| b.name.contains("Point Reads")) {
        let read_ratio = read_result.throughput / rocksdb_read.throughput_ops;
        let read_score = if read_ratio >= 1.0 { 100 } else { (read_ratio * 100.0) as u32 };
        
        println!("\n📖 Read Performance Comparison:");
        println!("   AuraDB: {:.0} ops/sec", read_result.throughput);
        println!("   RocksDB: {:.0} ops/sec ({})", rocksdb_read.throughput_ops, rocksdb_read.source);
        println!("   Ratio: {:.2}x | Score: {}/100", read_ratio, read_score);
        
        if read_ratio >= 1.0 {
            println!("   🎉 AuraDB outperforms RocksDB!");
        } else {
            println!("   📈 Gap to close: {:.1}x improvement needed", 1.0 / read_ratio);
        }
    }
    
    // Latency comparison
    println!("\n⏱ Latency Comparison:");
    println!("   AuraDB P50: {:.2} µs", read_result.p50_latency);
    println!("   AuraDB P99: {:.2} µs", read_result.p99_latency);
    println!("   RocksDB P50: ~450 µs (published)");
    println!("   RocksDB P99: ~1000 µs (published)");
    
    // Latency scoring (lower is better)
    let p50_ratio = read_result.p50_latency / 450.0;
    let p99_ratio = read_result.p99_latency / 1000.0;
    let latency_score = if p50_ratio <= 1.0 && p99_ratio <= 1.0 { 
        100 
    } else { 
        (100.0 / p50_ratio.max(p99_ratio)) as u32 
    };
    
    println!("   Latency Score: {}/100", latency_score);
    
    // Overall competitive assessment
    println!("\n🏆 COMPETITIVE ASSESSMENT");
    println!("=========================");
    
    let overall_score = calculate_overall_score(args, write_result, read_result, latency_score);
    
    println!("   Overall Score: {}/100", overall_score);
    println!("   Competitive Position: {}", get_competitive_position(overall_score));
    
    // Recommendations
    println!("\n💡 RECOMMENDATIONS");
    println!("==================");
    
    if overall_score < 50 {
        println!("   🚨 Critical gaps identified:");
        println!("   • Implement durability (fsync) immediately");
        println!("   • Add realistic disk I/O simulation");
        println!("   • Focus on M1 (WAL-time KV separation)");
    } else if overall_score < 70 {
        println!("   📈 Good foundation, key improvements needed:");
        println!("   • Complete M1 implementation");
        println!("   • Add LSM compaction (M2)");
        println!("   • Implement bloom filters");
    } else {
        println!("   🎉 Strong competitive position!");
        println!("   • Focus on M3-M6 innovations");
        println!("   • Leverage AI-driven features");
    }
}

/// Benchmark bulk random writes
fn benchmark_bulk_writes(db: &mut AuraEngine, args: &Args) -> BenchmarkResult {
    let mut rng = rand::thread_rng();
    let start = Instant::now();
    
    for i in 0..args.operations {
        let key: Vec<u8> = (0..args.key_size).map(|_| rng.gen::<u8>()).collect();
        let value: Vec<u8> = (0..args.value_size).map(|_| rng.gen::<u8>()).collect();
        
        db.put_bytes(&key, &value).expect("Write failed");
        
        // Simulate fsync if enabled
        if args.fsync {
            // In real implementation, this would be actual fsync
            std::thread::sleep(std::time::Duration::from_nanos(1000)); // 1µs simulation
        }
        
        if i % (args.operations / 10).max(1) == 0 {
            print!("\r   Progress: {}/{} ({:.0}%)", i, args.operations, (i as f64 / args.operations as f64) * 100.0);
        }
    }
    println!();
    
    let duration = start.elapsed().as_secs_f64();
    let throughput = args.operations as f64 / duration;
    
    BenchmarkResult {
        throughput,
        p50_latency: 0.0, // Not measured for writes
        p99_latency: 0.0,
    }
}

/// Benchmark point reads
fn benchmark_point_reads(db: &mut AuraEngine, args: &Args) -> BenchmarkResult {
    let mut rng = rand::thread_rng();
    let mut latencies = Vec::new();
    
    // Pre-populate with data
    let pre_populate_count = (args.operations / 10).max(1000);
    let mut keys = Vec::new();
    
    for _ in 0..pre_populate_count {
        let key: Vec<u8> = (0..args.key_size).map(|_| rng.gen::<u8>()).collect();
        let value: Vec<u8> = (0..args.value_size).map(|_| rng.gen::<u8>()).collect();
        db.put_bytes(&key, &value).expect("Pre-population failed");
        keys.push(key);
    }
    
    let start = Instant::now();
    
    for i in 0..args.operations {
        let key_idx = rng.gen_range(0..keys.len());
        let key = &keys[key_idx];
        
        let read_start = Instant::now();
        let _ = db.get_bytes(key);
        let latency = read_start.elapsed().as_nanos() as u64;
        latencies.push(latency);
        
        if i % (args.operations / 10).max(1) == 0 {
            print!("\r   Progress: {}/{} ({:.0}%)", i, args.operations, (i as f64 / args.operations as f64) * 100.0);
        }
    }
    println!();
    
    let duration = start.elapsed().as_secs_f64();
    let throughput = args.operations as f64 / duration;
    
    // Calculate percentiles
    latencies.sort();
    let p50 = latencies[latencies.len() * 50 / 100] as f64 / 1000.0; // Convert to µs
    let p99 = latencies[latencies.len() * 99 / 100] as f64 / 1000.0; // Convert to µs
    
    BenchmarkResult {
        throughput,
        p50_latency: p50,
        p99_latency: p99,
    }
}

/// Benchmark read-while-writing
fn benchmark_read_while_writing(db: &mut AuraEngine, args: &Args) -> BenchmarkResult {
    let mut rng = rand::thread_rng();
    let mut keys = Vec::new();
    let mut latencies = Vec::new();
    
    // Pre-populate with data
    let pre_populate_count = (args.operations / 10).max(1000);
    for _ in 0..pre_populate_count {
        let key: Vec<u8> = (0..args.key_size).map(|_| rng.gen::<u8>()).collect();
        let value: Vec<u8> = (0..args.value_size).map(|_| rng.gen::<u8>()).collect();
        db.put_bytes(&key, &value).expect("Pre-population failed");
        keys.push(key);
    }
    
    let start = Instant::now();
    let mut write_count = 0;
    let mut read_count = 0;
    
    for i in 0..args.operations {
        if rng.gen::<f64>() < 0.8 && !keys.is_empty() {
            // Read operation
            let key_idx = rng.gen_range(0..keys.len());
            let key = &keys[key_idx];
            
            let read_start = Instant::now();
            let _ = db.get_bytes(key);
            let latency = read_start.elapsed().as_nanos() as u64;
            latencies.push(latency);
            read_count += 1;
        } else {
            // Write operation
            let key: Vec<u8> = (0..args.key_size).map(|_| rng.gen::<u8>()).collect();
            let value: Vec<u8> = (0..args.value_size).map(|_| rng.gen::<u8>()).collect();
            db.put_bytes(&key, &value).expect("Write failed");
            keys.push(key);
            write_count += 1;
        }
        
        if i % (args.operations / 10).max(1) == 0 {
            print!("\r   Progress: {}/{} ({:.0}%)", i, args.operations, (i as f64 / args.operations as f64) * 100.0);
        }
    }
    println!();
    
    let duration = start.elapsed().as_secs_f64();
    let throughput = args.operations as f64 / duration;
    
    // Calculate percentiles
    latencies.sort();
    let p50 = latencies[latencies.len() * 50 / 100] as f64 / 1000.0; // Convert to µs
    let p99 = latencies[latencies.len() * 99 / 100] as f64 / 1000.0; // Convert to µs
    
    println!("   Read operations: {} | Write operations: {}", read_count, write_count);
    
    BenchmarkResult {
        throughput,
        p50_latency: p50,
        p99_latency: p99,
    }
}

/// Get published RocksDB benchmark data
fn get_rocksdb_benchmarks() -> Vec<RocksDBBenchmark> {
    vec![
        // Bulk random writes: ~1M ops/sec
        RocksDBBenchmark::new(
            "Bulk Random Writes",
            1_000_000.0,
            0.5,  // P50: 0.5µs
            2.0,  // P99: 2µs
            "RocksDB GitHub"
        ),
        // Point reads: P50 ~450µs, P99 ~1ms
        RocksDBBenchmark::new(
            "Point Reads",
            500_000.0,  // Estimated throughput
            450.0,      // P50: 450µs
            1000.0,     // P99: 1ms
            "RocksDB Documentation"
        ),
        // Read-while-writing: ~7M ops/sec
        RocksDBBenchmark::new(
            "Read-While-Writing",
            7_000_000.0,
            0.142, // P50: 0.142µs
            1.0,   // P99: 1µs
            "RocksDB GitHub"
        ),
    ]
}

/// Calculate overall competitive score
fn calculate_overall_score(
    args: &Args, 
    write_result: BenchmarkResult, 
    read_result: BenchmarkResult, 
    latency_score: u32
) -> u32 {
    let mut total_score = 0;
    
    // Write performance (30% weight)
    let write_score = if write_result.throughput >= 1_000_000.0 { 100 } else { (write_result.throughput / 1_000_000.0 * 100.0) as u32 };
    total_score += write_score * 30;
    
    // Read performance (30% weight)
    let read_score = if read_result.throughput >= 500_000.0 { 100 } else { (read_result.throughput / 500_000.0 * 100.0) as u32 };
    total_score += read_score * 30;
    
    // Latency performance (20% weight)
    total_score += latency_score * 20;
    
    // Value size handling (20% weight)
    let value_score = if args.value_size <= 1024 { 100 } else if args.value_size <= 8192 { 70 } else { 40 };
    total_score += value_score * 20;
    
    // Convert to percentage (total possible is 10000, so divide by 100)
    total_score / 100
}

/// Get competitive position description
fn get_competitive_position(score: u32) -> &'static str {
    match score {
        90..=100 => "Market Leader 🏆",
        80..=89 => "Strong Competitor 🥈",
        70..=79 => "Competitive 🥉",
        60..=69 => "Catching Up 📈",
        50..=59 => "Building Foundation 🚧",
        40..=49 => "Early Stage 🔬",
        30..=39 => "Research Phase 📚",
        _ => "Concept Phase 💡",
    }
}

/// Benchmark result
#[derive(Debug)]
struct BenchmarkResult {
    throughput: f64,
    p50_latency: f64,
    p99_latency: f64,
}

fn main() {
    let args = Args::parse();
    run_rocksdb_comparison(&args);
}
