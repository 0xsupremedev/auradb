use auradb::{EngineBuilder, AuraEngine};
use tempfile::tempdir;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a temporary directory for the database
    let temp_dir = tempdir()?;
    let db_path = temp_dir.path().join("auradb_data");
    
    println!("🚀 Starting AuraDB basic usage example...");
    println!("   Database path: {:?}", db_path);
    
    // Create the storage engine
    let engine = EngineBuilder::new()
        .path(&db_path)
        .build()?;
    
    println!("✅ Engine created successfully");
    
    // Basic operations
    println!("\n📝 Performing basic operations...");
    
    // Put some key-value pairs
    engine.put_str("name", "AuraDB")?;
    engine.put_str("version", "0.1.0")?;
    engine.put_str("language", "Rust")?;
    engine.put_str("type", "Storage Engine")?;
    engine.put_str("goal", "Surpass RocksDB")?;
    
    println!("   ✅ Put 5 key-value pairs");
    
    // Get values
    let name = engine.get_str("name")?;
    let version = engine.get_str("version")?;
    let language = engine.get_str("language")?;
    
    println!("   📖 Retrieved values:");
    println!("      name: {:?}", name);
    println!("      version: {:?}", version);
    println!("      language: {:?}", language);
    
    // Scan operation
    println!("\n🔍 Performing scan operation...");
    let results = engine.scan_str("a", "z")?;
    println!("   📊 Scan results (a-z): {} entries", results.len());
    for (key, value) in results.iter().take(3) {
        println!("      {}: {}", key, value);
    }
    
    // Delete operation
    println!("\n🗑️  Performing delete operation...");
    engine.delete_str("type")?;
    let deleted_value = engine.get_str("type")?;
    println!("   ✅ Deleted 'type' key, value: {:?}", deleted_value);
    
    // Batch operations
    println!("\n📦 Performing batch operations...");
    let mut batch = auradb::Batch::new();
    for i in 1..=5 {
        batch.add(auradb::storage::Entry::new(
            auradb::storage::Key::new(format!("batch_key_{}", i).into_bytes()),
            auradb::storage::Value::new(format!("batch_value_{}", i).into_bytes()),
            i as u64,
        ));
    }
    engine.write_batch(&batch)?;
    println!("   ✅ Wrote batch with {} operations", batch.len());
    
    // Verify batch results
    let batch_value = engine.get_str("batch_key_3")?;
    println!("   📖 Batch key 'batch_key_3': {:?}", batch_value);
    
    // Engine info
    println!("\nℹ️  Engine information:");
    println!("   Status: Active");
    println!("   Storage: In-memory HashMap (simplified)");
    println!("   Features: Basic KV operations, batch support, range scans");
    
    // Cleanup
    println!("\n🧹 Cleaning up...");
    // Note: In the simplified version, close() is not implemented
    // The engine will be cleaned up when it goes out of scope
    
    println!("✅ Example completed successfully!");
    println!("   Note: This is a simplified in-memory implementation.");
    println!("   Future versions will include WAL, value log, and SST files.");
    
    Ok(())
}
