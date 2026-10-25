use vectrust::prelude::*;
use rand::Rng;

#[test]
fn test_hnsw_basic_crud_and_search() -> vectrust::Result<()> {
    let db = Vectrust::in_memory();
    let config = CollectionConfig::builder(4, Metric::Euclidean)
        .index_type(IndexType::Hnsw)
        .build();
    let collection = db.create_collection("test_crud", config)?;

    assert_eq!(collection.count(), 0);
    assert!(collection.is_empty());

    // Insert 3 points
    let p1 = Point::new(1u64, vec![0.0, 0.0, 0.0, 0.0]);
    let p2 = Point::new(2u64, vec![1.0, 1.0, 1.0, 1.0]);
    let p3 = Point::new(3u64, vec![10.0, 10.0, 10.0, 10.0]);

    collection.insert(p1.clone())?;
    collection.insert(p2.clone())?;
    collection.insert(p3.clone())?;

    assert_eq!(collection.count(), 3);

    // Get point by ID
    let fetched = collection.get(1u64)?.expect("Point 1 should exist");
    assert_eq!(fetched.id, PointId::U64(1));
    assert_eq!(fetched.vector, vec![0.0, 0.0, 0.0, 0.0]);

    // Search nearest to [0.1, 0.1, 0.1, 0.1]
    let query = SearchQuery::new(vec![0.1, 0.1, 0.1, 0.1], 2);
    let results = collection.search(query)?;

    assert_eq!(results.len(), 2);
    assert_eq!(results[0].id, PointId::U64(1));
    assert_eq!(results[1].id, PointId::U64(2));

    // Delete point 1
    let deleted = collection.delete(1u64)?;
    assert!(deleted);
    assert_eq!(collection.count(), 2);
    assert!(collection.get(1u64)?.is_none());

    // Search again: closest should now be point 2
    let query = SearchQuery::new(vec![0.1, 0.1, 0.1, 0.1], 2);
    let results = collection.search(query)?;
    assert_eq!(results.len(), 2);
    assert_eq!(results[0].id, PointId::U64(2));
    assert_eq!(results[1].id, PointId::U64(3));

    Ok(())
}

#[test]
fn test_hnsw_recall_against_ground_truth() -> vectrust::Result<()> {
    let dim = 16;
    let num_points = 500;
    let mut rng = rand::thread_rng();

    // Create both HNSW and Flat collections to compare recall
    let hnsw_config = CollectionConfig::builder(dim, Metric::Cosine)
        .index_type(IndexType::Hnsw)
        .hnsw_config(HnswConfig {
            m: 16,
            m0: 32,
            ef_construction: 128,
            ef_search: 64,
        })
        .build();

    let flat_config = CollectionConfig::builder(dim, Metric::Cosine)
        .index_type(IndexType::Flat)
        .build();

    let db = Vectrust::in_memory();
    let hnsw_col = db.create_collection("hnsw_eval", hnsw_config)?;
    let flat_col = db.create_collection("flat_eval", flat_config)?;

    let mut points = Vec::with_capacity(num_points);
    for i in 0..num_points {
        let vec: Vec<f32> = (0..dim).map(|_| rng.gen_range(-1.0..1.0)).collect();
        points.push(Point::new(i as u64, vec));
    }

    hnsw_col.insert_batch(points.clone())?;
    flat_col.insert_batch(points)?;

    // Run 50 test queries and measure average recall@10
    let num_queries = 50;
    let k = 10;
    let mut total_recall = 0.0;

    for _ in 0..num_queries {
        let q_vec: Vec<f32> = (0..dim).map(|_| rng.gen_range(-1.0..1.0)).collect();
        let query = SearchQuery::new(q_vec, k);

        let hnsw_res = hnsw_col.search(query.clone())?;
        let flat_res = flat_col.search(query)?;

        assert_eq!(flat_res.len(), k);
        assert_eq!(hnsw_res.len(), k);

        let flat_ids: std::collections::HashSet<_> = flat_res.iter().map(|r| &r.id).collect();
        let matches = hnsw_res.iter().filter(|r| flat_ids.contains(&r.id)).count();
        let recall = matches as f64 / k as f64;
        total_recall += recall;
    }

    let avg_recall = total_recall / num_queries as f64;
    println!("Average Recall@{} over {} queries: {:.2}%", k, num_queries, avg_recall * 100.0);

    // HNSW with ef_search=64 and 500 vectors should achieve > 90% recall
    assert!(
        avg_recall >= 0.90,
        "HNSW recall was too low: {:.2}%",
        avg_recall * 100.0
    );

    Ok(())
}
