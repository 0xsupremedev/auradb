use criterion::{black_box, criterion_group, criterion_main, Criterion};
use auradb::{AuraEngine, Engine, EngineBuilder};
use tempfile::TempDir;

fn basic_operations_benchmark(c: &mut Criterion) {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("bench_db");
    
    let engine = EngineBuilder::new()
        .path(&db_path)
        .build()
        .unwrap();
    
    c.bench_function("put_small", |b| {
        b.iter(|| {
            engine.put_str("key", "value").unwrap();
        });
    });
    
    c.bench_function("get_small", |b| {
        b.iter(|| {
            engine.get_str("key").unwrap();
        });
    });
    
    c.bench_function("put_large", |b| {
        let large_value = "x".repeat(1024 * 1024); // 1MB
        b.iter(|| {
            engine.put_str("large_key", &large_value).unwrap();
        });
    });
    
    c.bench_function("batch_write", |b| {
        let mut batch = auradb::Batch::new();
        for i in 0..100 {
            batch.put(format!("batch_key_{}", i), format!("batch_value_{}", i));
        }
        b.iter(|| {
            engine.write_batch(&batch).unwrap();
        });
    });
}

criterion_group!(benches, basic_operations_benchmark);
criterion_main!(benches);
