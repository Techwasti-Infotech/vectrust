# Vectrust Public API Reference

This document provides a reference for the primary types and methods exposed by the `vectrust` library.

---

## 1. Prelude

Import the essential items into scope:

```rust
use vectrust::prelude::*;
```

The prelude exports:
- `Vectrust` (the primary embedded database engine)
- `Collection` (vector collection handle)
- `Point`, `PointId`
- `SearchQuery`, `SearchResult`
- `Filter`
- `Metric`, `IndexType`, `CollectionConfig`, `HnswConfig`
- `Result`, `VectrustError`

---

## 2. `Vectrust`

### Methods

#### `Vectrust::open(path: impl AsRef<Path>) -> Result<Self>`
Opens or creates a persistent Vectrust database directory. Automatically discovers and re-opens existing collections stored in `path/collections/`.

#### `Vectrust::in_memory() -> Self`
Initializes a fast, transient, in-memory database with no disk persistence.

#### `create_collection(&self, name: &str, config: CollectionConfig) -> Result<Collection>`
Creates a new named collection. Returns `VectrustError::CollectionAlreadyExists` if the name is already in use.

#### `get_collection(&self, name: &str) -> Option<Collection>`
Returns a thread-safe handle to an existing collection if present.

#### `get_or_create_collection(&self, name: &str, config: CollectionConfig) -> Result<Collection>`
Fetches the existing collection or creates a new one with the given config if not found.

#### `drop_collection(&self, name: &str) -> Result<bool>`
Deletes a collection and deletes all related files from disk.

#### `list_collections(&self) -> Vec<String>`
Returns a list of all active collection names.

#### `flush_all(&self) -> Result<()>`
Flushes and compacts all open collections to disk.

---

## 3. `Collection`

### Methods

#### `insert(&self, point: Point) -> Result<()>`
Inserts or replaces a vector point in the collection. Writes to the WAL in persistent mode.

#### `insert_batch(&self, points: Vec<Point>) -> Result<()>`
Efficiently inserts a batch of vector points.

#### `upsert(&self, point: Point) -> Result<()>`
Alias for `insert()`.

#### `get(&self, id: impl Into<PointId>) -> Result<Option<Point>>`
Retrieves a point by its unique identifier, including its vector and payload.

#### `delete(&self, id: impl Into<PointId>) -> Result<bool>`
Removes a point by its ID. Returns `true` if found and deleted, `false` otherwise.

#### `update_payload(&self, id: impl Into<PointId>, payload: Option<serde_json::Value>) -> Result<bool>`
Updates or removes the metadata payload of an existing vector point.

#### `search(&self, query: SearchQuery) -> Result<Vec<SearchResult>>`
Executes nearest neighbor vector search, applying optional metadata filters and ranking parameters.

#### `search_batch(&self, queries: Vec<SearchQuery>) -> Result<Vec<Vec<SearchResult>>>`
Executes multiple queries in parallel across worker threads using Rayon under a shared read lock.

#### `count(&self) -> usize`
Returns the number of active vectors currently stored in the collection.

#### `is_empty(&self) -> bool`
Returns `true` if the collection contains zero active points.

#### `compact(&self) -> Result<()>`
Flushes all in-memory state to an atomic snapshot (`snapshot.bin`) with CRC32 verification and truncates the WAL file.

#### `flush(&self) -> Result<()>`
Alias for `compact()`.

---

## 4. `Point` & `PointId`

### `PointId`
- `PointId::U64(u64)`: Fast unsigned integer identifier.
- `PointId::String(String)`: String/UUID identifier.
- Implements `From<u64>`, `From<usize>`, `From<&str>`, `From<String>`.

### `Point`
```rust
pub struct Point {
    pub id: PointId,
    pub vector: Vec<f32>,
    pub payload: Option<serde_json::Value>,
}

impl Point {
    pub fn new(id: impl Into<PointId>, vector: Vec<f32>) -> Self;
    pub fn with_payload(self, payload: serde_json::Value) -> Self;
}
```

---

## 5. `SearchQuery` & `SearchResult`

### `SearchQuery`
```rust
impl SearchQuery {
    pub fn new(vector: Vec<f32>, top_k: usize) -> Self;
    pub fn with_filter(self, filter: Filter) -> Self;
    pub fn with_ef_search(self, ef: usize) -> Self;
    pub fn include_vector(self, include: bool) -> Self;
    pub fn include_payload(self, include: bool) -> Self;
}
```

### `SearchResult`
```rust
pub struct SearchResult {
    pub id: PointId,
    pub distance: f32,
    pub score: f32,
    pub vector: Option<Vec<f32>>,
    pub payload: Option<serde_json::Value>,
}
```

---

## 6. `CollectionConfig` & `HnswConfig`

```rust
let config = CollectionConfig::builder(dimension, metric)
    .index_type(IndexType::Hnsw) // or IndexType::Flat
    .hnsw_config(HnswConfig {
        m: 16,
        m0: 32,
        ef_construction: 100,
        ef_search: 64,
    })
    .sync_on_write(false)
    .build();
```

---

## 7. Error Handling (`VectrustError`)

```rust
pub enum VectrustError {
    DimensionMismatch { expected: usize, got: usize },
    CollectionAlreadyExists(String),
    CollectionNotFound(String),
    PointNotFound(String),
    InvalidVectorData,
    ConfigError(String),
    IoError(std::io::Error),
    SerializationError(bincode::Error),
    JsonError(serde_json::Error),
    CorruptedData(String),
    LockError(String),
    FilterError(String),
    Internal(String),
}
```
All functions return `vectrust::Result<T>`.
