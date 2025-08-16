use std::time::Instant;
use hdrhistogram::Histogram;
use clap::Parser;
use auradb::AuraEngine;
use auradb::config::Config;

/// Benchmark configuration
#[derive(Parser, Debug)]
#[command(name = "AuraDB Benchmark")]
struct Args {
    /// Database path
    #[arg(short, long, default_value = "./benchmark_db")]
    db_path: String,
    /// Number of operations
    #[arg(short, long, default_value_t = 100000)]
    operations: usize,
    /// Key size in bytes
    #[arg(long, default_value_t = 16)]
    key_size: usize,
    /// Value size in bytes
    #[arg(long, default_value_t = 1024)]
    value_size: usize,
    /// Workload type (random, sequential)
    #[arg(long, default_value = "random")]
    workload: String,
    /// Whether to test large-value separation
    #[arg(long, default_value_t = false)]
    large_values: bool,
    /// Batch size for batch operations
    #[arg(long, default_value_t = 100)]
    batch_size: usize,
}

fn random_key(size: usize) -> Vec<u8> {
    (0..size).map(|_| rand::random::<u8>()).collect()
}

fn random_value(size: usize) -> Vec<u8> {
    (0..size).map(|_| rand::random::<u8>()).collect()
}

fn main() {
    let args = Args::parse();
    println!("🚀 AuraDB Benchmark");
    println!("===================");
    println!("Database path: {}", args.db_path);
    println!("Operations: {}", args.operations);
    println!("Key size: {} bytes", args.key_size);
    println!("Value size: {} bytes", args.value_size);
    println!("Workload: {}", args.workload);
    println!("Large values: {}", args.large_values);
    println!("Batch size: {}", args.batch_size);

    // Engine init
    let config = Config::default();
    let engine = AuraEngine::new(config).expect("Failed to create engine");
    println!("✅ Engine created successfully");

    // Histogram for latency
    let mut hist = Histogram::<u64>::new(3).unwrap();
    
    println!("🔄 Running random workload...");
    let start = Instant::now();
    let mut i = 0;
    
    while i < args.operations {
        if args.batch_size > 1 {
            // --- Batch write path ---
            let mut batch = Vec::with_capacity(args.batch_size);
            for _ in 0..args.batch_size.min(args.operations - i) {
                let key = random_key(args.key_size);
                let value = random_value(args.value_size);
                batch.push((key, value));
            }
            
            let t0 = Instant::now();
            engine.write_batch(&batch).unwrap(); // you'll implement write_batch in engine
            let elapsed = t0.elapsed().as_nanos() as u64;
            hist.record(elapsed).unwrap();
            i += args.batch_size;
        } else {
            // --- Single op path ---
            let key = random_key(args.key_size);
            let value = random_value(args.value_size);
            
            let t0 = Instant::now();
            if rand::random::<f64>() < 0.33 {
                engine.put_bytes(&key, &value).unwrap();
            } else {
                let _ = engine.get_bytes(&key);
            }
            let elapsed = t0.elapsed().as_nanos() as u64;
            hist.record(elapsed).unwrap();
            i += 1;
        }
    }
    
    let total_duration = start.elapsed().as_secs_f64();
    let throughput = args.operations as f64 / total_duration;
    
    println!(" ✅ Workload completed:");
    println!(" Duration: {:.2} s", total_duration);
    println!(" Throughput: {:.2} ops/sec", throughput);
    println!(" Latency p50: {} µs", hist.value_at_quantile(0.50) / 1000);
    println!(" Latency p95: {} µs", hist.value_at_quantile(0.95) / 1000);
    println!(" Latency p99: {} µs", hist.value_at_quantile(0.99) / 1000);
    
    println!("✅ Benchmark completed successfully!");
}
