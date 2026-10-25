use serde_json::json;
use vectrust::prelude::*;

#[test]
fn test_exhaustive_filter_operators() -> vectrust::Result<()> {
    let db = Vectrust::in_memory();
    let config = CollectionConfig::builder(2, Metric::Euclidean).build();
    let col = db.create_collection("filters_matrix", config)?;

    col.insert(
        Point::new(1u64, vec![1.0, 1.0]).with_payload(json!({
            "age": 25,
            "status": "active",
            "score": 85.5,
            "tags": ["rust", "ai", "fast"],
            "profile": { "verified": true, "country": "US" }
        })),
    )?;

    col.insert(
        Point::new(2u64, vec![2.0, 2.0]).with_payload(json!({
            "age": 42,
            "status": "pending",
            "score": 92.0,
            "tags": ["python", "slow"],
            "profile": { "verified": false, "country": "DE" }
        })),
    )?;

    col.insert(
        Point::new(3u64, vec![3.0, 3.0]).with_payload(json!({
            "age": 19,
            "status": "inactive",
            "score": 60.0,
            "tags": ["rust", "embedded"],
            "profile": { "verified": true, "country": "JP" }
        })),
    )?;

    let base_vec = vec![0.0, 0.0];

    // 1. Eq operator
    let res = col.search(SearchQuery::new(base_vec.clone(), 5).with_filter(Filter::eq("status", "active")))?;
    assert_eq!(res.len(), 1);
    assert_eq!(res[0].id, PointId::U64(1));

    // 2. Ne operator
    let res = col.search(SearchQuery::new(base_vec.clone(), 5).with_filter(Filter::ne("status", "active")))?;
    assert_eq!(res.len(), 2);

    // 3. Gt & Gte operators
    let res = col.search(SearchQuery::new(base_vec.clone(), 5).with_filter(Filter::gt("age", 25.0)))?;
    assert_eq!(res.len(), 1);
    assert_eq!(res[0].id, PointId::U64(2));

    let res = col.search(SearchQuery::new(base_vec.clone(), 5).with_filter(Filter::gte("age", 25.0)))?;
    assert_eq!(res.len(), 2);

    // 4. Lt & Lte operators
    let res = col.search(SearchQuery::new(base_vec.clone(), 5).with_filter(Filter::lt("age", 25.0)))?;
    assert_eq!(res.len(), 1);
    assert_eq!(res[0].id, PointId::U64(3));

    let res = col.search(SearchQuery::new(base_vec.clone(), 5).with_filter(Filter::lte("age", 25.0)))?;
    assert_eq!(res.len(), 2);

    // 5. In & Not(In) operators
    let res = col.search(SearchQuery::new(base_vec.clone(), 5).with_filter(Filter::in_vals("profile.country", vec![json!("US"), json!("JP")])))?;
    assert_eq!(res.len(), 2);

    let res = col.search(SearchQuery::new(base_vec.clone(), 5).with_filter(Filter::not(Filter::in_vals("profile.country", vec![json!("US"), json!("JP")]))))?;
    assert_eq!(res.len(), 1);
    assert_eq!(res[0].id, PointId::U64(2));

    // 6. Contains operator on JSON Array
    let res = col.search(SearchQuery::new(base_vec.clone(), 5).with_filter(Filter::contains("tags", "rust")))?;
    assert_eq!(res.len(), 2);

    // 7. Nested Dot Path
    let res = col.search(SearchQuery::new(base_vec.clone(), 5).with_filter(Filter::eq("profile.verified", true)))?;
    assert_eq!(res.len(), 2);

    // 8. Boolean Combination: AND, OR, NOT
    let complex_filter = Filter::and(vec![
        Filter::contains("tags", "rust"),
        Filter::or(vec![
            Filter::eq("profile.country", "US"),
            Filter::lt("age", 20.0),
        ]),
        Filter::not(Filter::eq("status", "pending")),
    ]);

    let res = col.search(SearchQuery::new(base_vec.clone(), 5).with_filter(complex_filter))?;
    assert_eq!(res.len(), 2);

    // 9. Filter that matches 0 points
    let res = col.search(SearchQuery::new(base_vec.clone(), 5).with_filter(Filter::eq("status", "non_existent")))?;
    assert!(res.is_empty());

    Ok(())
}
