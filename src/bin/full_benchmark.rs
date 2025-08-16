use std::time::Instant;
use std::fs;
use std::path::Path;
use auradb::AuraEngine;
use auradb::config::Config;
use rand::Rng;

fn run_workload(label: &str, operations: usize, key_size: usize, value_size: usize, read_ratio: f64) {
    println!("\n📊 Running workload: {label}");
    println!("   Operations: {} | Key size: {}B | Value size: {}B | Read ratio: {:.0}%", 
             operations, key_size, value_size, read_ratio * 100.0);
    
    let config = Config::default();
    let mut db = AuraEngine::new(config).expect("DB init failed");

    let mut rng = rand::thread_rng();
    let mut keys: Vec<Vec<u8>> = Vec::with_capacity(operations);

    let start = Instant::now();

    for i in 0..operations {
        let key: Vec<u8> = (0..key_size).map(|_| rng.gen::<u8>()).collect();
        let val: Vec<u8> = (0..value_size).map(|_| rng.gen::<u8>()).collect();

        if rng.gen::<f64>() < read_ratio && !keys.is_empty() {
            let k = &keys[rng.gen_range(0..keys.len())];
            let _ = db.get_bytes(k);
        } else {
            db.put_bytes(&key, &val).expect("Put failed");
            keys.push(key);
        }
        
        // Progress indicator for long-running workloads
        if i % (operations / 10).max(1) == 0 {
            print!("\r   Progress: {}/{} ({:.0}%)", i, operations, (i as f64 / operations as f64) * 100.0);
        }
    }
    println!(); // Clear progress line

    let duration = start.elapsed().as_secs_f64();

    let throughput_ops = operations as f64 / duration;
    let throughput_mb = (operations as f64 * value_size as f64) / (1024.0 * 1024.0 * duration);

    println!("✅ Duration: {:.2} s", duration);
    println!("🚀 Throughput: {:.0} ops/sec | {:.2} MB/s", throughput_ops, throughput_mb);
    println!("💾 Disk size: {:.2} MB", folder_size("./benchmark_db") as f64 / (1024.0 * 1024.0));
}

fn run_latency_workload(label: &str, operations: usize, key_size: usize, value_size: usize) {
    println!("\n⏱ Running latency workload: {label}");
    println!("   Operations: {} | Key size: {}B | Value size: {}B", operations, key_size, value_size);
    
    let config = Config::default();
    let db = AuraEngine::new(config).expect("DB init failed");

    let mut rng = rand::thread_rng();
    let mut latencies = Vec::with_capacity(operations);

    // Pre-populate with some data
    println!("   📝 Pre-populating database...");
    for i in 0..(operations / 10) {
        let key: Vec<u8> = (0..key_size).map(|_| rng.gen::<u8>()).collect();
        let val: Vec<u8> = (0..value_size).map(|_| rng.gen::<u8>()).collect();
        db.put_bytes(&key, &val).expect("Put failed");
    }

    println!("   🔍 Measuring read latencies...");
    for i in 0..operations {
        let key: Vec<u8> = (0..key_size).map(|_| rng.gen::<u8>()).collect();
        
        let start = Instant::now();
        let _ = db.get_bytes(&key);
        let elapsed = start.elapsed().as_nanos() as u64;
        latencies.push(elapsed);
        
        if i % (operations / 10).max(1) == 0 {
            print!("\r   Progress: {}/{} ({:.0}%)", i, operations, (i as f64 / operations as f64) * 100.0);
        }
    }
    println!(); // Clear progress line

    // Calculate percentiles
    latencies.sort();
    let p50 = latencies[latencies.len() * 50 / 100];
    let p95 = latencies[latencies.len() * 95 / 100];
    let p99 = latencies[latencies.len() * 99 / 100];
    let p999 = latencies[latencies.len() * 999 / 1000];

    println!("✅ Latency percentiles:");
    println!("   p50:  {} ns ({:.2} µs)", p50, p50 as f64 / 1000.0);
    println!("   p95:  {} ns ({:.2} µs)", p95, p95 as f64 / 1000.0);
    println!("   p99:  {} ns ({:.2} µs)", p99, p99 as f64 / 1000.0);
    println!("   p99.9: {} ns ({:.2} µs)", p999, p999 as f64 / 1000.0);
}

fn run_mixed_workload(label: &str, operations: usize, key_size: usize, value_size: usize, write_ratio: f64) {
    println!("\n🔀 Running mixed workload: {label}");
    println!("   Operations: {} | Key size: {}B | Value size: {}B | Write ratio: {:.0}%", 
             operations, key_size, value_size, write_ratio * 100.0);
    
    let config = Config::default();
    let db = AuraEngine::new(config).expect("DB init failed");

    let mut rng = rand::thread_rng();
    let mut keys: Vec<Vec<u8>> = Vec::new();
    let mut write_count = 0;
    let mut read_count = 0;

    let start = Instant::now();

    for i in 0..operations {
        if rng.gen::<f64>() < write_ratio || keys.is_empty() {
            // Write operation
            let key: Vec<u8> = (0..key_size).map(|_| rng.gen::<u8>()).collect();
            let val: Vec<u8> = (0..value_size).map(|_| rng.gen::<u8>()).collect();
            db.put_bytes(&key, &val).expect("Put failed");
            keys.push(key);
            write_count += 1;
        } else {
            // Read operation
            let k = &keys[rng.gen_range(0..keys.len())];
            let _ = db.get_bytes(k);
            read_count += 1;
        }
        
        if i % (operations / 10).max(1) == 0 {
            print!("\r   Progress: {}/{} ({:.0}%)", i, operations, (i as f64 / operations as f64) * 100.0);
        }
    }
    println!(); // Clear progress line

    let duration = start.elapsed().as_secs_f64();

    let total_throughput = operations as f64 / duration;
    let write_throughput = write_count as f64 / duration;
    let read_throughput = read_count as f64 / duration;

    println!("✅ Duration: {:.2} s", duration);
    println!("🚀 Total throughput: {:.0} ops/sec", total_throughput);
    println!("📝 Write throughput: {:.0} ops/sec ({:.0}%)", write_throughput, (write_count as f64 / operations as f64) * 100.0);
    println!("📖 Read throughput: {:.0} ops/sec ({:.0}%)", read_throughput, (read_count as f64 / operations as f64) * 100.0);
}

fn folder_size<P: AsRef<Path>>(path: P) -> u64 {
    match fs::read_dir(path) {
        Ok(entries) => {
            entries.filter_map(|f| {
                f.ok().and_then(|entry| {
                    if let Ok(file_type) = entry.file_type() {
                        if file_type.is_dir() {
                            Some(folder_size(entry.path()))
                        } else {
                            entry.metadata().ok().map(|m| m.len())
                        }
                    } else {
                        None
                    }
                })
            }).sum()
        }
        Err(_) => 0, // Return 0 if directory doesn't exist or can't be read
    }
}

fn main() {
    println!("🚀 Full AuraDB Benchmark Suite");
    println!("================================");
    
    // Clean up any existing database
    fs::remove_dir_all("./benchmark_db").ok();
    
    // Test 1: Small KV writes (1 KB) — measures raw ops/sec
    run_workload("Small Writes (1 KB)", 100_000, 16, 1024, 0.0);
    fs::remove_dir_all("./benchmark_db").ok();

    // Test 2: Medium KV writes (8 KB) — measures MB/s scaling
    run_workload("Medium Writes (8 KB)", 50_000, 16, 8192, 0.0);
    fs::remove_dir_all("./benchmark_db").ok();

    // Test 3: Large KV writes (64 KB) — tests WAL-time KV separation benefit
    run_workload("Large Writes (64 KB)", 10_000, 16, 65536, 0.0);
    fs::remove_dir_all("./benchmark_db").ok();

    // Test 4: Read-heavy workload (80% reads, 20% writes) — tests cache efficiency
    run_mixed_workload("Read Heavy (80% reads)", 50_000, 16, 1024, 0.2);
    fs::remove_dir_all("./benchmark_db").ok();

    // Test 5: Write-heavy workload (80% writes, 20% reads)
    run_mixed_workload("Write Heavy (80% writes)", 50_000, 16, 1024, 0.8);
    fs::remove_dir_all("./benchmark_db").ok();

    // Test 6: Latency profiling for small values
    run_latency_workload("Latency Profile (1 KB)", 100_000, 16, 1024);
    fs::remove_dir_all("./benchmark_db").ok();

    println!("\n🎯 Benchmark Summary");
    println!("====================");
    println!("✅ All workloads completed successfully!");
    println!("📊 Check the results above to identify:");
    println!("   • Raw speed bottlenecks (CPU vs I/O)");
    println!("   • Value size impact on performance");
    println!("   • Read/write ratio effects");
    println!("   • Latency distribution characteristics");
    println!("   • Expected WAL-time KV separation benefits for large values");
}
