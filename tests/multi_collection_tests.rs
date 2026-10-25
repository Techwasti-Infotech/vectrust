use tempfile::tempdir;
use vectrust::prelude::*;

#[test]
fn test_multiple_collections_with_different_metrics() -> vectrust::Result<()> {
    let tmp = tempdir().unwrap();
    let db_path = tmp.path().to_path_buf();

    let db = Vectrust::open(&db_path)?;

    // 1. Cosine Collection
    let col_cosine = db.create_collection(
        "embeddings_cosine",
        CollectionConfig::builder(4, Metric::Cosine).build(),
    )?;

    // 2. Euclidean Collection
    let col_euclidean = db.create_collection(
        "images_l2",
        CollectionConfig::builder(4, Metric::Euclidean).build(),
    )?;

    // 3. Dot Product Collection
    let col_dot = db.create_collection(
        "recommendations_mips",
        CollectionConfig::builder(4, Metric::DotProduct).build(),
    )?;

    // 4. Manhattan Collection
    let col_manhattan = db.create_collection(
        "geo_l1",
        CollectionConfig::builder(4, Metric::Manhattan).build(),
    )?;

    assert_eq!(db.list_collections().len(), 4);

    let sample_vec = vec![0.5, 0.5, 0.5, 0.5];
    col_cosine.insert(Point::new(1u64, sample_vec.clone()))?;
    col_euclidean.insert(Point::new(1u64, sample_vec.clone()))?;
    col_dot.insert(Point::new(1u64, sample_vec.clone()))?;
    col_manhattan.insert(Point::new(1u64, sample_vec.clone()))?;

    // Search across each
    let res_cos = col_cosine.search(SearchQuery::new(sample_vec.clone(), 1))?;
    let res_euc = col_euclidean.search(SearchQuery::new(sample_vec.clone(), 1))?;
    let res_dot = col_dot.search(SearchQuery::new(sample_vec.clone(), 1))?;
    let res_man = col_manhattan.search(SearchQuery::new(sample_vec.clone(), 1))?;

    assert_eq!(res_cos[0].distance, 0.0);
    assert_eq!(res_euc[0].distance, 0.0);
    assert_eq!(res_man[0].distance, 0.0);
    assert_eq!(res_dot[0].score, 1.0); // 4 * 0.25 = 1.0

    // Flush and commit all
    db.flush_all()?;

    // Drop one collection
    let dropped = db.drop_collection("geo_l1")?;
    assert!(dropped);
    assert_eq!(db.list_collections().len(), 3);
    assert!(db.get_collection("geo_l1").is_none());

    // Other collections remain unaffected
    assert_eq!(col_cosine.count(), 1);
    assert_eq!(col_euclidean.count(), 1);
    assert_eq!(col_dot.count(), 1);

    Ok(())
}

#[test]
fn test_high_dimensional_vectors_simd() -> vectrust::Result<()> {
    let db = Vectrust::in_memory();
    // 512 dimensions (common for small CLIP/BERT models)
    let dim = 512;
    let config = CollectionConfig::builder(dim, Metric::Cosine)
        .index_type(IndexType::Hnsw)
        .build();
    let col = db.create_collection("high_dim", config)?;

    let mut v1 = vec![0.0; dim];
    v1[0] = 1.0;
    v1[1] = 0.5;

    let mut v2 = vec![0.0; dim];
    v2[dim - 1] = 1.0;
    v2[dim - 2] = 0.5;

    col.insert(Point::new(101u64, v1.clone()))?;
    col.insert(Point::new(102u64, v2.clone()))?;

    let results = col.search(SearchQuery::new(v1, 2))?;
    assert_eq!(results.len(), 2);
    assert_eq!(results[0].id, PointId::U64(101));
    assert_eq!(results[1].id, PointId::U64(102));
    assert!(results[0].distance < results[1].distance);

    Ok(())
}
