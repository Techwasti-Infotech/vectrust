//! Quickstart Example for Vectrust
//!
//! Demonstrates the minimal setup to create an in-memory collection,
//! insert vectors with payloads, and perform approximate nearest neighbor search.
//!
//! Run with: `cargo run --example quickstart`

use serde_json::json;
use vectrust::prelude::*;

fn main() -> vectrust::Result<()> {
    println!("=== Vectrust Quickstart Demo ===");

    // 1. Initialize an in-memory database instance
    let db = Vectrust::in_memory();

    // 2. Configure a collection with 4-dimensional vectors and Cosine distance
    let config = CollectionConfig::builder(4, Metric::Cosine)
        .index_type(IndexType::Hnsw)
        .build();

    let collection = db.create_collection("tech_articles", config)?;
    println!("Created collection 'tech_articles' (dim=4, metric=Cosine)");

    // 3. Insert documents with embeddings and rich metadata payloads
    let articles = vec![
        Point::new(1u64, vec![0.9, 0.1, 0.2, 0.0]).with_payload(json!({
            "title": "Getting Started with Rust 2024",
            "author": "Alice",
            "category": "systems",
            "reads": 1540
        })),
        Point::new(2u64, vec![0.85, 0.15, 0.1, 0.05]).with_payload(json!({
            "title": "High Performance Vector Search in Rust",
            "author": "Bob",
            "category": "systems",
            "reads": 4200
        })),
        Point::new(3u64, vec![0.1, 0.8, 0.7, 0.2]).with_payload(json!({
            "title": "Machine Learning with Transformers",
            "author": "Charlie",
            "category": "ai",
            "reads": 3100
        })),
        Point::new(4u64, vec![0.05, 0.9, 0.65, 0.15]).with_payload(json!({
            "title": "Fine-Tuning Open Source LLMs",
            "author": "Alice",
            "category": "ai",
            "reads": 5800
        })),
        Point::new(5u64, vec![0.2, 0.1, 0.05, 0.95]).with_payload(json!({
            "title": "Building Reactive UIs with WebAssembly",
            "author": "David",
            "category": "frontend",
            "reads": 980
        })),
    ];

    println!("Inserting {} articles...", articles.len());
    collection.insert_batch(articles)?;
    println!("Current collection vector count: {}", collection.count());

    // 4. Perform a semantic vector search
    // Query represents an interest in systems programming and Rust: [0.88, 0.12, 0.15, 0.02]
    let query_vector = vec![0.88, 0.12, 0.15, 0.02];
    println!("\nSearching top 3 nearest articles for query vector: {:?}", query_vector);

    let query = SearchQuery::new(query_vector.clone(), 3)
        .include_payload(true)
        .include_vector(false);

    let results = collection.search(query)?;

    for (rank, res) in results.iter().enumerate() {
        let payload = res.payload.as_ref().unwrap();
        println!(
            "  #{}: ID={:<2} | Similarity Score: {:.4} | Title: \"{}\" (by {})",
            rank + 1,
            res.id,
            res.score,
            payload["title"].as_str().unwrap(),
            payload["author"].as_str().unwrap()
        );
    }

    // 5. Perform vector search with structured metadata filter
    println!("\nSearching with Filter: category == 'ai'...");
    let filtered_query = SearchQuery::new(vec![0.1, 0.8, 0.7, 0.2], 2)
        .with_filter(Filter::eq("category", "ai"));

    let ai_results = collection.search(filtered_query)?;
    for (rank, res) in ai_results.iter().enumerate() {
        let payload = res.payload.as_ref().unwrap();
        println!(
            "  #{}: ID={:<2} | Score: {:.4} | Title: \"{}\"",
            rank + 1,
            res.id,
            res.score,
            payload["title"].as_str().unwrap()
        );
    }

    Ok(())
}
