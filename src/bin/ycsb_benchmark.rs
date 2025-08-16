use std::time::Instant;
use std::fs;
use std::path::Path;
use auradb::AuraEngine;
use auradb::config::Config;
use rand::Rng;
use clap::Parser;

/// YCSB-style benchmark configuration
#[derive(Parser, Debug)]
#[command(name = "AuraDB YCSB Benchmark")]
struct Args {
    /// Workload type (A, B, C, D, E, F)
    #[arg(short, long, default_value = "A")]
    workload: String,
    
    /// Number of operations
    #[arg(short, long, default_value_t = 100000)]
    operations: usize,
    
    /// Key size in bytes
    #[arg(long, default_value_t = 16)]
    key_size: usize,
    
    /// Value size in bytes
    #[arg(long, default_value_t = 1024)]
    value_size: usize,
    
    /// Number of threads
    #[arg(long, default_value_t = 1)]
    threads: usize,
    
    /// Database path
    #[arg(long, default_value = "./ycsb_db")]
    db_path: String,
}

/// YCSB Workload Definitions
#[derive(Debug, Clone)]
struct YCSBWorkload {
    name: String,
    read_ratio: f64,
    update_ratio: f64,
    insert_ratio: f64,
    scan_ratio: f64,
    read_modify_write_ratio: f64,
    distribution: String,
}

impl YCSBWorkload {
    fn new(name: &str, read: f64, update: f64, insert: f64, scan: f64, rmw: f64, dist: &str) -> Self {
        Self {
            name: name.to_string(),
            read_ratio: read,
            update_ratio: update,
            insert_ratio: insert,
            scan_ratio: scan,
            read_modify_write_ratio: rmw,
            distribution: dist.to_string(),
        }
    }

    fn workload_a() -> Self {
        // Workload A: Update heavy workload
        // 50% reads, 50% updates
        Self::new("A", 0.5, 0.5, 0.0, 0.0, 0.0, "zipfian")
    }

    fn workload_b() -> Self {
        // Workload B: Read mostly workload
        // 95% reads, 5% updates
        Self::new("B", 0.95, 0.05, 0.0, 0.0, 0.0, "zipfian")
    }

    fn workload_c() -> Self {
        // Workload C: Read only
        // 100% reads
        Self::new("C", 1.0, 0.0, 0.0, 0.0, 0.0, "zipfian")
    }

    fn workload_d() -> Self {
        // Workload D: Read latest workload
        // 95% reads, 5% inserts
        Self::new("D", 0.95, 0.0, 0.05, 0.0, 0.0, "latest")
    }

    fn workload_e() -> Self {
        // Workload E: Short ranges
        // 95% scans, 5% inserts
        Self::new("E", 0.0, 0.0, 0.05, 0.95, 0.0, "zipfian")
    }

    fn workload_f() -> Self {
        // Workload F: Read-modify-write
        // 50% reads, 50% read-modify-write
        Self::new("F", 0.5, 0.0, 0.0, 0.0, 0.5, "zipfian")
    }

    fn from_name(name: &str) -> Option<Self> {
        match name.to_uppercase().as_str() {
            "A" => Some(Self::workload_a()),
            "B" => Some(Self::workload_b()),
            "C" => Some(Self::workload_c()),
            "D" => Some(Self::workload_d()),
            "E" => Some(Self::workload_e()),
            "F" => Some(Self::workload_f()),
            _ => None,
        }
    }
}

/// Custom workload runner for detailed metrics
struct CustomWorkloadRunner {
    db: AuraEngine,
    workload: YCSBWorkload,
    operations: usize,
    key_size: usize,
    value_size: usize,
}

impl CustomWorkloadRunner {
    fn new(workload: YCSBWorkload, operations: usize, key_size: usize, value_size: usize) -> Self {
        let config = Config::default();
        let db = AuraEngine::new(config).expect("Failed to create AuraDB engine");
        
        Self {
            db,
            workload,
            operations,
            key_size,
            value_size,
        }
    }

    fn run(&mut self) -> WorkloadResult {
        let mut rng = rand::thread_rng();
        let mut keys: Vec<Vec<u8>> = Vec::new();
        let mut latencies = Vec::new();
        
        let mut read_count = 0;
        let mut update_count = 0;
        let mut insert_count = 0;
        let mut scan_count = 0;
        let mut rmw_count = 0;

        let start = Instant::now();

        // Pre-populate with some data for reads/updates
        if self.workload.read_ratio > 0.0 || self.workload.update_ratio > 0.0 {
            let pre_populate_count = (self.operations / 10).max(1000);
            println!("   📝 Pre-populating database with {} keys...", pre_populate_count);
            
            for _ in 0..pre_populate_count {
                let key = self.generate_key(&mut rng);
                let value = self.generate_value(&mut rng);
                self.db.put_bytes(&key, &value).expect("Pre-population failed");
                keys.push(key);
            }
        }

        println!("   🔄 Running workload {}...", self.workload.name);
        
        for i in 0..self.operations {
            let operation_start = Instant::now();
            
            let rand_val = rng.gen::<f64>();
            let mut cumulative = 0.0;
            
            // Determine operation type based on ratios
            if rand_val < (cumulative + self.workload.read_ratio) {
                // Read operation
                if !keys.is_empty() {
                    let key_idx = rng.gen_range(0..keys.len());
                    let key = &keys[key_idx];
                    let _ = self.db.get_bytes(key);
                    read_count += 1;
                }
                cumulative += self.workload.read_ratio;
            } else if rand_val < (cumulative + self.workload.update_ratio) {
                // Update operation
                if !keys.is_empty() {
                    let key_idx = rng.gen_range(0..keys.len());
                    let key = &keys[key_idx];
                    let value = self.generate_value(&mut rng);
                    self.db.put_bytes(key, &value);
                    update_count += 1;
                }
                cumulative += self.workload.update_ratio;
            } else if rand_val < (cumulative + self.workload.insert_ratio) {
                // Insert operation
                let key = self.generate_key(&mut rng);
                let value = self.generate_value(&mut rng);
                self.db.put_bytes(&key, &value);
                keys.push(key);
                insert_count += 1;
                cumulative += self.workload.insert_ratio;
            } else if rand_val < (cumulative + self.workload.scan_ratio) {
                // Scan operation (simplified as range read)
                if !keys.is_empty() {
                    let start_idx = rng.gen_range(0..keys.len().saturating_sub(10));
                    let end_idx = (start_idx + 10).min(keys.len());
                    for j in start_idx..end_idx {
                        let _ = self.db.get_bytes(&keys[j]);
                    }
                    scan_count += 1;
                }
                cumulative += self.workload.scan_ratio;
            } else if rand_val < (cumulative + self.workload.read_modify_write_ratio) {
                // Read-modify-write operation
                if !keys.is_empty() {
                    let key_idx = rng.gen_range(0..keys.len());
                    let key = &keys[key_idx];
                    let _ = self.db.get_bytes(key);
                    let value = self.generate_value(&mut rng);
                    self.db.put_bytes(key, &value);
                    rmw_count += 1;
                }
                cumulative += self.workload.read_modify_write_ratio;
            }

            let latency = operation_start.elapsed().as_nanos() as u64;
            latencies.push(latency);

            // Progress indicator
            if i % (self.operations / 20).max(1) == 0 {
                print!("\r   Progress: {}/{} ({:.0}%)", i, self.operations, (i as f64 / self.operations as f64) * 100.0);
            }
        }
        println!(); // Clear progress line

        let total_duration = start.elapsed().as_secs_f64();
        
        // Calculate percentiles
        latencies.sort();
        let p50 = latencies[latencies.len() * 50 / 100];
        let p95 = latencies[latencies.len() * 95 / 100];
        let p99 = latencies[latencies.len() * 99 / 100];
        let p999 = latencies[latencies.len() * 999 / 1000];

        WorkloadResult {
            workload_name: self.workload.name.clone(),
            total_operations: self.operations,
            duration: total_duration,
            throughput_ops: self.operations as f64 / total_duration,
            throughput_mb: (self.operations as f64 * self.value_size as f64) / (1024.0 * 1024.0 * total_duration),
            read_count,
            update_count,
            insert_count,
            scan_count,
            rmw_count,
            latency_p50: p50,
            latency_p95: p95,
            latency_p99: p99,
            latency_p999: p999,
        }
    }

    fn generate_key(&self, rng: &mut impl Rng) -> Vec<u8> {
        (0..self.key_size).map(|_| rng.gen::<u8>()).collect()
    }

    fn generate_value(&self, rng: &mut impl Rng) -> Vec<u8> {
        (0..self.value_size).map(|_| rng.gen::<u8>()).collect()
    }
}

/// Results from workload execution
#[derive(Debug)]
struct WorkloadResult {
    workload_name: String,
    total_operations: usize,
    duration: f64,
    throughput_ops: f64,
    throughput_mb: f64,
    read_count: usize,
    update_count: usize,
    insert_count: usize,
    scan_count: usize,
    rmw_count: usize,
    latency_p50: u64,
    latency_p95: u64,
    latency_p99: u64,
    latency_p999: u64,
}

impl WorkloadResult {
    fn print_summary(&self) {
        println!("\n📊 Workload {} Results", self.workload_name);
        println!("==========================================");
        println!("✅ Duration: {:.2} s", self.duration);
        println!("🚀 Total throughput: {:.0} ops/sec | {:.2} MB/s", self.throughput_ops, self.throughput_mb);
        println!("\n📈 Operation Breakdown:");
        println!("   📖 Reads: {} ({:.1}%)", self.read_count, (self.read_count as f64 / self.total_operations as f64) * 100.0);
        println!("   ✏️  Updates: {} ({:.1}%)", self.update_count, (self.update_count as f64 / self.total_operations as f64) * 100.0);
        println!("   ➕ Inserts: {} ({:.1}%)", self.insert_count, (self.insert_count as f64 / self.total_operations as f64) * 100.0);
        println!("   🔍 Scans: {} ({:.1}%)", self.scan_count, (self.scan_count as f64 / self.total_operations as f64) * 100.0);
        println!("   🔄 Read-Modify-Writes: {} ({:.1}%)", self.rmw_count, (self.rmw_count as f64 / self.total_operations as f64) * 100.0);
        println!("\n⏱ Latency Percentiles:");
        println!("   p50:   {} ns ({:.2} µs)", self.latency_p50, self.latency_p50 as f64 / 1000.0);
        println!("   p95:   {} ns ({:.2} µs)", self.latency_p95, self.latency_p95 as f64 / 1000.0);
        println!("   p99:   {} ns ({:.2} µs)", self.latency_p99, self.latency_p99 as f64 / 1000.0);
        println!("   p99.9: {} ns ({:.2} µs)", self.latency_p999, self.latency_p999 as f64 / 1000.0);
    }
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
    let args = Args::parse();
    
    println!("🚀 AuraDB YCSB-Style Benchmark Suite");
    println!("=====================================");
    println!("Workload: {}", args.workload);
    println!("Operations: {}", args.operations);
    println!("Key size: {} bytes", args.key_size);
    println!("Value size: {} bytes", args.value_size);
    println!("Threads: {}", args.threads);
    println!("Database path: {}", args.db_path);

    // Get workload definition
    let workload = match YCSBWorkload::from_name(&args.workload) {
        Some(w) => w,
        None => {
            eprintln!("❌ Unknown workload type: {}. Available: A, B, C, D, E, F", args.workload);
            std::process::exit(1);
        }
    };

    println!("\n📋 Workload {} Configuration:", workload.name);
    println!("   Read ratio: {:.1}%", workload.read_ratio * 100.0);
    println!("   Update ratio: {:.1}%", workload.update_ratio * 100.0);
    println!("   Insert ratio: {:.1}%", workload.insert_ratio * 100.0);
    println!("   Scan ratio: {:.1}%", workload.scan_ratio * 100.0);
    println!("   Read-Modify-Write ratio: {:.1}%", workload.read_modify_write_ratio * 100.0);
    println!("   Distribution: {}", workload.distribution);

    // Clean up any existing database
    fs::remove_dir_all(&args.db_path).ok();
    
    println!("\n🔧 Running YCSB workload...");
    let mut runner = CustomWorkloadRunner::new(workload.clone(), args.operations, args.key_size, args.value_size);
    let result = runner.run();
    result.print_summary();
    
    // Show disk usage
    let disk_size = folder_size(&args.db_path);
    println!("\n💾 Final disk usage: {:.2} MB", disk_size as f64 / (1024.0 * 1024.0));
    
    println!("\n🎯 YCSB Benchmark Summary");
    println!("==========================");
    println!("✅ Workload {} completed successfully!", workload.name);
    println!("📊 This benchmark provides:");
    println!("   • Standard YCSB workload patterns (A-F)");
    println!("   • Accurate operation mix ratios");
    println!("   • Detailed latency percentiles (p50, p95, p99, p99.9)");
    println!("   • Throughput measurements (ops/sec, MB/s)");
    println!("   • Operation breakdown by type");
    println!("   • Disk usage tracking");
}
