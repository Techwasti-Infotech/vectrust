# Getting Started with Vectrust

This guide covers everything you need to start embedding and querying vectors with **Vectrust**.

---

## 1. Installation

Add `vectrust` and `serde_json` to your `Cargo.toml`:

```toml
[dependencies]
vectrust = { path = "path/to/vectrust" } # or version = "0.1.0" once published
serde_json = "1.0"
```

---

## 2. In-Memory vs Persistent Storage

### In-Memory Database (Ephemeral)
Ideal for testing, scratchpads, caching layers, or CLI tools:

```rust
use vectrust::prelude::*;

let db = Vectrust::in_memory();
```

### Persistent On-Disk Database (Durable)
Stores snapshots and WAL logs on disk:

```rust
use vectrust::prelude::*;

let db = Vectrust::open("./my_vector_db")?;
```

---

## 3. Creating a Collection

Each collection stores vectors of a fixed dimension, compared with a specified metric.

```rust
use vectrust::prelude::*;

// Configure a collection with 128 dimensions and Cosine similarity
let config = CollectionConfig::builder(128, Metric::Cosine)
    .index_type(IndexType::Hnsw)
    .hnsw_config(HnswConfig {
        m: 16,                // Max connections per node
        m0: 32,               // Max connections at layer 0 (typically 2 * m)
        ef_construction: 128, // Quality of graph during build
        ef_search: 64,        // Search beam width
    })
    .sync_on_write(false)     // True for instant fsync durability
    .build();

let collection = db.get_or_create_collection("articles", config)?;
```

---

## 4. Inserting Points and Metadata

A `Point` consists of:
1. `id`: A unique `PointId` (accepts `u64`, `&str`, or `String`).
2. `vector`: A `Vec<f32>` matching the collection's dimension.
3. `payload`: An optional JSON metadata payload (`serde_json::Value`).

### Single Insertion
```rust
use serde_json::json;

collection.insert(
    Point::new("doc_1", vec![0.1; 128])
        .with_payload(json!({
            "title": "Introduction to Rust",
            "category": "programming",
            "year": 2024
        }))
)?;
```

### Batch Ingestion (Recommended for Speed)
```rust
let points = vec![
    Point::new(101u64, vec![0.2; 128]).with_payload(json!({"tag": "ai"})),
    Point::new(102u64, vec![0.4; 128]).with_payload(json!({"tag": "database"})),
];

collection.insert_batch(points)?;
```

---

## 5. Searching Vectors

Execute nearest neighbor queries using `SearchQuery`:

```rust
let query_vector = vec![0.15; 128];

// Find top 5 closest vectors
let query = SearchQuery::new(query_vector, 5)
    .include_payload(true)
    .include_vector(false);

let results = collection.search(query)?;

for r in results {
    println!("ID: {}, Distance: {:.4}, Score: {:.4}", r.id, r.distance, r.score);
    if let Some(payload) = r.payload {
        println!("Metadata: {}", payload);
    }
}
```

---

## 6. Querying with Metadata Filters

Combine semantic vector search with exact metadata filters:

```rust
let query = SearchQuery::new(vec![0.15; 128], 3)
    .with_filter(Filter::and(vec![
        Filter::eq("category", "programming"),
        Filter::gte("year", 2023.0),
    ]));

let results = collection.search(query)?;
```

---

## 7. Point Management: Get, Update, Delete

### Get Point by ID
```rust
if let Some(point) = collection.get("doc_1")? {
    println!("Found vector of length: {}", point.vector.len());
}
```

### Update Metadata Payload
```rust
collection.update_payload("doc_1", Some(json!({"status": "archived"})))?;
```

### Delete Point
```rust
let removed = collection.delete("doc_1")?;
assert!(removed);
```

---

## 8. Compaction & Clean Snapshotting

To commit pending Write-Ahead Log records into a clean, compact snapshot file:

```rust
collection.compact()?;
```
