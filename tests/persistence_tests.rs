use serde_json::json;
use tempfile::tempdir;
use vectrust::prelude::*;

#[test]
fn test_persistence_roundtrip_and_compaction() -> vectrust::Result<()> {
    let tmp = tempdir().unwrap();
    let db_path = tmp.path().to_path_buf();

    let config = CollectionConfig::builder(4, Metric::Euclidean)
        .index_type(IndexType::Hnsw)
        .build();

    // 1. Open DB and create collection
    {
        let db = Vectrust::open(&db_path)?;
        let collection = db.create_collection("docs", config.clone())?;

        collection.insert(
            Point::new(101u64, vec![1.0, 2.0, 3.0, 4.0])
                .with_payload(json!({"title": "Rust Vector Database", "views": 1200})),
        )?;
        collection.insert(
            Point::new(102u64, vec![5.0, 6.0, 7.0, 8.0])
                .with_payload(json!({"title": "Distributed Systems", "views": 3400})),
        )?;

        assert_eq!(collection.count(), 2);
        collection.compact()?;
    }

    // 2. Re-open DB from disk and verify data persisted in snapshot
    {
        let db = Vectrust::open(&db_path)?;
        let collection = db.get_collection("docs").expect("Collection should exist on reopen");

        assert_eq!(collection.count(), 2);

        let pt = collection.get(101u64)?.expect("Point 101 should exist");
        assert_eq!(pt.vector, vec![1.0, 2.0, 3.0, 4.0]);
        assert_eq!(pt.payload.unwrap()["views"], 1200);

        // Perform search
        let query = SearchQuery::new(vec![1.1, 2.1, 3.1, 4.1], 1);
        let results = collection.search(query)?;
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, PointId::U64(101));
    }

    // 3. Test WAL replay without compaction (crash simulation)
    {
        let db = Vectrust::open(&db_path)?;
        let collection = db.get_collection("docs").unwrap();

        // Add 2 more points and delete 1, without calling compact()!
        collection.insert(
            Point::new(103u64, vec![9.0, 10.0, 11.0, 12.0])
                .with_payload(json!({"title": "Embedded Engines", "views": 5600})),
        )?;
        collection.insert(
            Point::new(104u64, vec![0.5, 0.5, 0.5, 0.5])
                .with_payload(json!({"title": "SIMD Acceleration", "views": 9000})),
        )?;
        collection.delete(102u64)?;

        assert_eq!(collection.count(), 3);
    }

    // 4. Reopen again: WAL should replay and reconstruct exact state
    {
        let db = Vectrust::open(&db_path)?;
        let collection = db.get_collection("docs").unwrap();

        assert_eq!(collection.count(), 3);
        assert!(collection.get(102u64)?.is_none(), "Point 102 should be deleted");
        assert!(collection.get(103u64)?.is_some(), "Point 103 should exist");
        assert!(collection.get(104u64)?.is_some(), "Point 104 should exist");

        let pt = collection.get(104u64)?.unwrap();
        assert_eq!(pt.payload.unwrap()["title"], "SIMD Acceleration");
    }

    Ok(())
}
