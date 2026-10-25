//! Advanced Metadata Filtering Example
//!
//! Demonstrates complex predicate queries combined with vector similarity search.
//!
//! Run with: `cargo run --example metadata_filtering`

use serde_json::json;
use vectrust::prelude::*;

fn main() -> vectrust::Result<()> {
    println!("=== Vectrust Metadata Filtering Demo ===");

    let db = Vectrust::in_memory();
    let config = CollectionConfig::builder(8, Metric::Euclidean).build();
    let collection = db.create_collection("ecommerce_catalog", config)?;

    // Insert 6 product items with structured specifications
    let products = vec![
        Point::new("sku_macbook", vec![0.1, 0.2, 0.8, 0.4, 0.5, 0.1, 0.0, 0.9]).with_payload(json!({
            "name": "MacBook Pro 16",
            "category": "laptop",
            "price": 2499.00,
            "rating": 4.8,
            "in_stock": true,
            "specs": { "ram_gb": 32, "storage_gb": 1000, "brand": "Apple" },
            "tags": ["premium", "developer", "creator"]
        })),
        Point::new("sku_thinkpad", vec![0.12, 0.22, 0.78, 0.42, 0.48, 0.11, 0.02, 0.88]).with_payload(json!({
            "name": "ThinkPad X1 Carbon",
            "category": "laptop",
            "price": 1699.00,
            "rating": 4.6,
            "in_stock": true,
            "specs": { "ram_gb": 16, "storage_gb": 512, "brand": "Lenovo" },
            "tags": ["business", "developer", "lightweight"]
        })),
        Point::new("sku_gaming_pc", vec![0.15, 0.25, 0.75, 0.45, 0.60, 0.15, 0.0, 0.92]).with_payload(json!({
            "name": "Alienware Aurora R15",
            "category": "desktop",
            "price": 2899.00,
            "rating": 4.4,
            "in_stock": false,
            "specs": { "ram_gb": 64, "storage_gb": 2000, "brand": "Dell" },
            "tags": ["gaming", "workstation", "high-end"]
        })),
        Point::new("sku_ipad", vec![0.4, 0.6, 0.2, 0.1, 0.3, 0.8, 0.7, 0.1]).with_payload(json!({
            "name": "iPad Pro 12.9",
            "category": "tablet",
            "price": 1099.00,
            "rating": 4.7,
            "in_stock": true,
            "specs": { "ram_gb": 8, "storage_gb": 256, "brand": "Apple" },
            "tags": ["creative", "portable", "touch"]
        })),
        Point::new("sku_galaxy_tab", vec![0.42, 0.58, 0.22, 0.12, 0.32, 0.78, 0.68, 0.12]).with_payload(json!({
            "name": "Galaxy Tab S9 Ultra",
            "category": "tablet",
            "price": 999.00,
            "rating": 4.5,
            "in_stock": true,
            "specs": { "ram_gb": 12, "storage_gb": 256, "brand": "Samsung" },
            "tags": ["oled", "portable", "touch"]
        })),
        Point::new("sku_budget_chromebook", vec![0.11, 0.21, 0.79, 0.41, 0.49, 0.12, 0.01, 0.89]).with_payload(json!({
            "name": "Chromebook Spin 714",
            "category": "laptop",
            "price": 549.00,
            "rating": 4.1,
            "in_stock": true,
            "specs": { "ram_gb": 8, "storage_gb": 128, "brand": "Acer" },
            "tags": ["budget", "student"]
        })),
    ];

    collection.insert_batch(products)?;
    println!("Inserted 6 electronics catalog items.\n");

    let query_vector = vec![0.11, 0.21, 0.79, 0.41, 0.50, 0.11, 0.01, 0.90]; // developer laptop vector

    // --- Scenario 1: Developer Laptop with RAM >= 16 GB and In-Stock ---
    println!("Query 1: Developer laptop (RAM >= 16GB, in_stock == true)");
    let filter1 = Filter::and(vec![
        Filter::eq("in_stock", true),
        Filter::gte("specs.ram_gb", 16.0),
    ]);
    let results1 = collection.search(SearchQuery::new(query_vector.clone(), 5).with_filter(filter1))?;
    for res in &results1 {
        let p = res.payload.as_ref().unwrap();
        println!(
            "  -> {} | RAM: {}GB | Price: ${:.2} | Distance: {:.4}",
            p["name"].as_str().unwrap(),
            p["specs"]["ram_gb"],
            p["price"].as_f64().unwrap(),
            res.distance
        );
    }

    // --- Scenario 2: Budget Query (Price <= $1500) OR Brand == Apple ---
    println!("\nQuery 2: Price <= $1500 OR brand == 'Apple'");
    let filter2 = Filter::or(vec![
        Filter::lte("price", 1500.00),
        Filter::eq("specs.brand", "Apple"),
    ]);
    let results2 = collection.search(SearchQuery::new(query_vector.clone(), 5).with_filter(filter2))?;
    for res in &results2 {
        let p = res.payload.as_ref().unwrap();
        println!(
            "  -> {} (Brand: {}) | Price: ${:.2}",
            p["name"].as_str().unwrap(),
            p["specs"]["brand"].as_str().unwrap(),
            p["price"].as_f64().unwrap()
        );
    }

    // --- Scenario 3: Categorical Selection with `In` and Rating threshold ---
    println!("\nQuery 3: Category IN ['tablet', 'desktop'] AND rating >= 4.5");
    let filter3 = Filter::and(vec![
        Filter::in_vals("category", vec![json!("tablet"), json!("desktop")]),
        Filter::gte("rating", 4.5),
    ]);
    let results3 = collection.search(SearchQuery::new(query_vector, 5).with_filter(filter3))?;
    for res in &results3 {
        let p = res.payload.as_ref().unwrap();
        println!(
            "  -> {} (Category: {}) | Rating: {:.1}",
            p["name"].as_str().unwrap(),
            p["category"].as_str().unwrap(),
            p["rating"].as_f64().unwrap()
        );
    }

    Ok(())
}
