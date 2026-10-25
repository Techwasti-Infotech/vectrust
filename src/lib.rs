//! # Vectrust
//!
//! **Vectrust** is a high-performance, embedded vector database engine written in pure Rust.
//! It is designed for maximum speed, memory efficiency, seamless concurrency, and zero-configuration setup.
//!
//! ## Core Features
//! - **State-of-the-art ANN Indexing**: Hierarchical Navigable Small World (HNSW) graph index with sub-millisecond query latency.
//! - **Exact Ground Truth (Flat) Index**: Brute-force linear scan baseline with Rayon-powered multi-threaded execution.
//! - **Multiple Distance Metrics**: Euclidean (L2), Cosine Similarity, Dot Product / Inner Product, Manhattan (L1) with auto-vectorized loops.
//! - **Structured Metadata Filtering**: Pre-filtering and filter-aware traversal supporting nested JSON paths, equality, ranges, boolean `AND`/`OR`/`NOT`.
//! - **Crash-Resilient Persistence**: Append-only Write-Ahead Log (WAL) and atomic snapshots with CRC32 integrity verification.
//! - **Concurrent & Thread-Safe**: Read-heavy non-blocking architecture using `parking_lot::RwLock` and parallel batch query processing via `rayon`.
//! - **Embedded & Lightweight**: Zero external services, zero C++ dependencies, embeds directly into your Rust binary.
//!
//! ## Quick Start
//! ```rust
//! use vectrust::prelude::*;
//! use serde_json::json;
//!
//! # fn main() -> vectrust::Result<()> {
//! // 1. Open an embedded database instance (or use Vectrust::in_memory())
//! let db = Vectrust::in_memory();
//!
//! // 2. Create a collection with 4-dimensional vectors and Cosine metric
//! let config = CollectionConfig::builder(4, Metric::Cosine).build();
//! let collection = db.create_collection("articles", config)?;
//!
//! // 3. Insert vector points with structured metadata payload
//! collection.insert(
//!     Point::new(1u64, vec![0.1, 0.8, 0.2, 0.5])
//!         .with_payload(json!({"topic": "rust", "stars": 150}))
//! )?;
//! collection.insert(
//!     Point::new(2u64, vec![0.9, 0.1, 0.4, 0.2])
//!         .with_payload(json!({"topic": "python", "stars": 80}))
//! )?;
//!
//! // 4. Search with metadata filtering
//! let query = SearchQuery::new(vec![0.1, 0.75, 0.2, 0.45], 5)
//!     .with_filter(Filter::eq("topic", "rust"));
//!
//! let results = collection.search(query)?;
//! for r in results {
//!     println!("Found ID: {} with similarity score: {:.4}", r.id, r.score);
//! }
//! # Ok(())
//! # }
//! ```

pub mod collection;
pub mod db;
pub mod error;
pub mod filter;
pub mod index;
pub mod metric;
pub mod storage;
pub mod types;

// Flattened top-level re-exports for maximum ergonomics
pub use collection::Collection;
pub use db::Vectrust;
pub use error::{Result, VectrustError};
pub use filter::Filter;
pub use metric::Metric;
pub use types::{
    CollectionConfig, CollectionConfigBuilder, HnswConfig, IndexType, Point, PointId,
    SearchQuery, SearchResult,
};

/// A convenience prelude module to import all essential types with a single glob:
/// `use vectrust::prelude::*;`
pub mod prelude {
    pub use crate::collection::Collection;
    pub use crate::db::Vectrust;
    pub use crate::error::{Result, VectrustError};
    pub use crate::filter::Filter;
    pub use crate::metric::Metric;
    pub use crate::types::{
        CollectionConfig, CollectionConfigBuilder, HnswConfig, IndexType, Point, PointId,
        SearchQuery, SearchResult,
    };
}
