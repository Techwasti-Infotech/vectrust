use serde_json::json;
use vectrust::prelude::*;

#[test]
fn test_complex_metadata_filtering() -> vectrust::Result<()> {
    let db = Vectrust::in_memory();
    let config = CollectionConfig::builder(3, Metric::Cosine)
        .index_type(IndexType::Hnsw)
        .build();
    let collection = db.create_collection("catalog", config)?;

    // Insert 5 items
    collection.insert(
        Point::new("item_1", vec![1.0, 0.0, 0.0])
            .with_payload(json!({
                "category": "laptop",
                "price": 999.0,
                "in_stock": true,
                "tags": ["gaming", "portable"],
                "specs": {"ram": 16, "cpu": "intel"}
            })),
    )?;

    collection.insert(
        Point::new("item_2", vec![0.9, 0.1, 0.0])
            .with_payload(json!({
                "category": "laptop",
                "price": 1499.0,
                "in_stock": false,
                "tags": ["workstation", "portable"],
                "specs": {"ram": 32, "cpu": "amd"}
            })),
    )?;

    collection.insert(
        Point::new("item_3", vec![0.8, 0.2, 0.1])
            .with_payload(json!({
                "category": "phone",
                "price": 799.0,
                "in_stock": true,
                "tags": ["mobile", "oled"],
                "specs": {"ram": 8, "cpu": "arm"}
            })),
    )?;

    collection.insert(
        Point::new("item_4", vec![0.1, 0.9, 0.0])
            .with_payload(json!({
                "category": "monitor",
                "price": 349.0,
                "in_stock": true,
                "tags": ["4k", "gaming"],
                "specs": {"hz": 144}
            })),
    )?;

    collection.insert(
        Point::new("item_5", vec![0.0, 0.1, 0.9])
            .with_payload(json!({
                "category": "keyboard",
                "price": 129.0,
                "in_stock": true,
                "tags": ["mechanical", "rgb"]
            })),
    )?;

    // Query 1: Find laptops with RAM >= 16 and in_stock == true
    let filter = Filter::and(vec![
        Filter::eq("category", "laptop"),
        Filter::eq("in_stock", true),
        Filter::gte("specs.ram", 16.0),
    ]);

    let query = SearchQuery::new(vec![1.0, 0.0, 0.0], 5).with_filter(filter);
    let results = collection.search(query)?;

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].id, PointId::String("item_1".into()));

    // Query 2: Find items where price < 500 OR category is "phone"
    let filter2 = Filter::or(vec![
        Filter::lt("price", 500.0),
        Filter::eq("category", "phone"),
    ]);

    let query2 = SearchQuery::new(vec![1.0, 0.0, 0.0], 5).with_filter(filter2);
    let results2 = collection.search(query2)?;

    let found_ids: Vec<String> = results2.iter().map(|r| r.id.to_string()).collect();
    assert!(found_ids.contains(&"item_3".to_string()));
    assert!(found_ids.contains(&"item_4".to_string()));
    assert!(found_ids.contains(&"item_5".to_string()));
    assert!(!found_ids.contains(&"item_1".to_string()));
    assert!(!found_ids.contains(&"item_2".to_string()));

    // Query 3: Inclusion filter `In`
    let filter3 = Filter::in_vals(
        "category",
        vec![json!("phone"), json!("keyboard")],
    );
    let query3 = SearchQuery::new(vec![0.5, 0.5, 0.5], 5).with_filter(filter3);
    let results3 = collection.search(query3)?;
    assert_eq!(results3.len(), 2);

    Ok(())
}
