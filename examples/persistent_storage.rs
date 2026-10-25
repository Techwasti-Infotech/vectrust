//! Persistent Storage and Crash Recovery Example
//!
//! Demonstrates Vectrust's embedded on-disk storage engine:
//! - Creating a persistent database directory
//! - Atomic snapshot creation with CRC32 integrity verification
//! - Write-Ahead Log (WAL) append-only durability
//! - Automatic crash recovery and state reconstruction
//!
//! Run with: `cargo run --example persistent_storage`

use std::fs;
use serde_json::json;
use vectrust::prelude::*;

fn main() -> vectrust::Result<()> {
    println!("=== Vectrust Persistent Storage Demo ===");

    let db_path = "./vectrust_data_demo";

    // Clean up demo directory if it exists from earlier runs
    if fs::metadata(db_path).is_ok() {
        let _ = fs::remove_dir_all(db_path);
    }

    let config = CollectionConfig::builder(4, Metric::Euclidean)
        .index_type(IndexType::Hnsw)
        .sync_on_write(true) // Ensure immediate fsync
        .build();

    // -------------------------------------------------------------
    // Phase 1: Open DB, create collection, insert items, save snapshot
    // -------------------------------------------------------------
    println!("\n[Phase 1] Opening database at '{}'...", db_path);
    {
        let db = Vectrust::open(db_path)?;
        let collection = db.create_collection("customer_embeddings", config.clone())?;

        println!("Inserting initial vector points...");
        collection.insert(
            Point::new("cust_001", vec![0.1, 0.2, 0.3, 0.4])
                .with_payload(json!({"tier": "gold", "country": "US", "spend": 4500})),
        )?;
        collection.insert(
            Point::new("cust_002", vec![0.5, 0.6, 0.7, 0.8])
                .with_payload(json!({"tier": "silver", "country": "UK", "spend": 1200})),
        )?;

        println!("Collection count: {}", collection.count());
        println!("Triggering compact() to save atomic snapshot to disk...");
        collection.compact()?;
        println!("Phase 1 completed. Simulating graceful database close.\n");
    }

    // -------------------------------------------------------------
    // Phase 2: Reopen DB and simulate sudden crash after WAL mutations
    // -------------------------------------------------------------
    println!("[Phase 2] Reopening database from disk...");
    {
        let db = Vectrust::open(db_path)?;
        let collection = db.get_collection("customer_embeddings").expect("Collection exists");
        println!("Recovered {} points from snapshot.", collection.count());

        // Perform mutations to Write-Ahead Log WITHOUT compacting!
        println!("Appending new points and deleting cust_001 in WAL...");
        collection.insert(
            Point::new("cust_003", vec![0.2, 0.3, 0.4, 0.5])
                .with_payload(json!({"tier": "platinum", "country": "DE", "spend": 12000})),
        )?;
        collection.insert(
            Point::new("cust_004", vec![0.9, 0.8, 0.7, 0.6])
                .with_payload(json!({"tier": "bronze", "country": "US", "spend": 300})),
        )?;
        collection.delete("cust_001")?;

        println!("Active in-memory count before crash: {}", collection.count());
        println!("Simulating ungraceful process crash (WAL not compacted)!\n");
        // Scope ends without calling collection.compact() or db.flush_all()
    }

    // -------------------------------------------------------------
    // Phase 3: Reopen database and verify automatic WAL recovery
    // -------------------------------------------------------------
    println!("[Phase 3] Reopening database after simulated crash...");
    {
        let db = Vectrust::open(db_path)?;
        let collection = db.get_collection("customer_embeddings").expect("Collection exists");

        println!("Collection recovered! Count: {}", collection.count());
        assert_eq!(collection.count(), 3);

        // Verify cust_001 was deleted
        assert!(collection.get("cust_001")?.is_none(), "cust_001 was deleted");

        // Verify cust_003 and cust_004 were reconstructed from WAL
        let p3 = collection.get("cust_003")?.expect("cust_003 recovered");
        let p4 = collection.get("cust_004")?.expect("cust_004 recovered");

        println!(
            "  -> Recovered cust_003: tier={}, spend=${}",
            p3.payload.as_ref().unwrap()["tier"],
            p3.payload.as_ref().unwrap()["spend"]
        );
        println!(
            "  -> Recovered cust_004: tier={}, spend=${}",
            p4.payload.as_ref().unwrap()["tier"],
            p4.payload.as_ref().unwrap()["spend"]
        );

        // Perform search on recovered data
        let results = collection.search(SearchQuery::new(vec![0.2, 0.3, 0.4, 0.5], 2))?;
        println!("\nNearest customer to [0.2, 0.3, 0.4, 0.5]:");
        for (i, r) in results.iter().enumerate() {
            println!(
                "  #{}: ID={} | Distance={:.4} | Payload={:?}",
                i + 1,
                r.id,
                r.distance,
                r.payload
            );
        }

        // Clean snapshot
        collection.compact()?;
        println!("\nDatabase state clean and fully compacted!");
    }

    // Clean up demo directory
    let _ = fs::remove_dir_all(db_path);
    println!("\nPersistent Storage Demo completed successfully!");

    Ok(())
}
