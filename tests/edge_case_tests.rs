use serde_json::json;
use vectrust::prelude::*;

#[test]
fn test_dimension_mismatch_error() -> vectrust::Result<()> {
    let db = Vectrust::in_memory();
    let config = CollectionConfig::builder(4, Metric::Cosine).build();
    let col = db.create_collection("dim_check", config)?;

    // Insert valid 4D point
    col.insert(Point::new(1u64, vec![0.1, 0.2, 0.3, 0.4]))?;

    // Attempt to insert 3D point (should error)
    let err = col.insert(Point::new(2u64, vec![0.1, 0.2, 0.3])).unwrap_err();
    match err {
        VectrustError::DimensionMismatch { expected, got } => {
            assert_eq!(expected, 4);
            assert_eq!(got, 3);
        }
        other => panic!("Expected DimensionMismatch error, got {:?}", other),
    }

    // Attempt to search with 5D query vector (should error)
    let query_err = col.search(SearchQuery::new(vec![0.1, 0.2, 0.3, 0.4, 0.5], 5)).unwrap_err();
    match query_err {
        VectrustError::DimensionMismatch { expected, got } => {
            assert_eq!(expected, 4);
            assert_eq!(got, 5);
        }
        other => panic!("Expected DimensionMismatch error, got {:?}", other),
    }

    Ok(())
}

#[test]
fn test_zero_vector_and_cosine_safety() -> vectrust::Result<()> {
    let db = Vectrust::in_memory();
    let config = CollectionConfig::builder(3, Metric::Cosine).build();
    let col = db.create_collection("zero_vecs", config)?;

    // Point with all zeros
    col.insert(Point::new(10u64, vec![0.0, 0.0, 0.0]))?;
    col.insert(Point::new(20u64, vec![1.0, 0.0, 0.0]))?;

    // Query with zero vector: should not panic or produce NaN
    let results = col.search(SearchQuery::new(vec![0.0, 0.0, 0.0], 2))?;
    assert_eq!(results.len(), 2);
    for r in results {
        assert!(!r.distance.is_nan(), "Distance must not be NaN");
        assert!(!r.score.is_nan(), "Score must not be NaN");
    }

    Ok(())
}

#[test]
fn test_search_more_than_existing_points() -> vectrust::Result<()> {
    let db = Vectrust::in_memory();
    let config = CollectionConfig::builder(2, Metric::Euclidean).build();
    let col = db.create_collection("small", config)?;

    col.insert(Point::new(1u64, vec![1.0, 1.0]))?;
    col.insert(Point::new(2u64, vec![2.0, 2.0]))?;

    // Request top 50 when only 2 points exist
    let results = col.search(SearchQuery::new(vec![0.0, 0.0], 50))?;
    assert_eq!(results.len(), 2);
    assert_eq!(results[0].id, PointId::U64(1));
    assert_eq!(results[1].id, PointId::U64(2));

    Ok(())
}

#[test]
fn test_search_empty_collection() -> vectrust::Result<()> {
    let db = Vectrust::in_memory();
    let config = CollectionConfig::builder(4, Metric::Cosine).build();
    let col = db.create_collection("empty", config)?;

    let results = col.search(SearchQuery::new(vec![0.5, 0.5, 0.5, 0.5], 10))?;
    assert!(results.is_empty());
    assert_eq!(col.count(), 0);
    assert!(col.is_empty());

    Ok(())
}

#[test]
fn test_string_ids_and_payload_crud() -> vectrust::Result<()> {
    let db = Vectrust::in_memory();
    let config = CollectionConfig::builder(2, Metric::Euclidean).build();
    let col = db.create_collection("string_ids", config)?;

    let doc_id = "doc-uuid-98765-abcdef";
    col.insert(
        Point::new(doc_id, vec![3.0, 4.0])
            .with_payload(json!({"status": "draft", "version": 1})),
    )?;

    // Get point
    let pt = col.get(doc_id)?.expect("Point should exist");
    assert_eq!(pt.id, PointId::String(doc_id.to_string()));
    assert_eq!(pt.payload.unwrap()["status"], "draft");

    // Update payload
    let updated = col.update_payload(doc_id, Some(json!({"status": "published", "version": 2})))?;
    assert!(updated);

    let pt2 = col.get(doc_id)?.unwrap();
    assert_eq!(pt2.payload.unwrap()["status"], "published");

    // Remove payload (set to None)
    let cleared = col.update_payload(doc_id, None)?;
    assert!(cleared);

    let pt3 = col.get(doc_id)?.unwrap();
    assert!(pt3.payload.is_none());

    // Update payload for non-existent ID
    let missing_update = col.update_payload("does-not-exist", Some(json!({"x": 1})))?;
    assert!(!missing_update);

    // Delete non-existent ID
    let missing_delete = col.delete("does-not-exist")?;
    assert!(!missing_delete);

    // Delete existing ID
    let deleted = col.delete(doc_id)?;
    assert!(deleted);
    assert_eq!(col.count(), 0);

    Ok(())
}

#[test]
fn test_duplicate_and_identical_vectors() -> vectrust::Result<()> {
    let db = Vectrust::in_memory();
    let config = CollectionConfig::builder(3, Metric::Euclidean).build();
    let col = db.create_collection("duplicates", config)?;

    // Insert 5 points with identical vectors but different IDs
    for i in 1..=5 {
        col.insert(Point::new(i as u64, vec![1.5, 2.5, 3.5]))?;
    }

    assert_eq!(col.count(), 5);

    let results = col.search(SearchQuery::new(vec![1.5, 2.5, 3.5], 5))?;
    assert_eq!(results.len(), 5);
    for r in results {
        assert_eq!(r.distance, 0.0);
    }

    Ok(())
}
