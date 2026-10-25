use std::fmt;
use serde::{Deserialize, Serialize};
use crate::metric::Metric;
use crate::filter::Filter;

/// A unique identifier for a vector point in Vectrust.
/// Supports both fast 64-bit unsigned integers and descriptive strings.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PointId {
    U64(u64),
    String(String),
}

impl PointId {
    pub fn as_str(&self) -> String {
        match self {
            PointId::U64(v) => v.to_string(),
            PointId::String(s) => s.clone(),
        }
    }
}

impl fmt::Display for PointId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PointId::U64(val) => write!(f, "{}", val),
            PointId::String(val) => write!(f, "{}", val),
        }
    }
}

impl From<u64> for PointId {
    fn from(val: u64) -> Self {
        PointId::U64(val)
    }
}

impl From<usize> for PointId {
    fn from(val: usize) -> Self {
        PointId::U64(val as u64)
    }
}

impl From<&str> for PointId {
    fn from(val: &str) -> Self {
        PointId::String(val.to_string())
    }
}

impl From<String> for PointId {
    fn from(val: String) -> Self {
        PointId::String(val)
    }
}

/// A point represents a vector along with its unique ID and optional metadata payload.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Point {
    pub id: PointId,
    pub vector: Vec<f32>,
    pub payload: Option<serde_json::Value>,
}

impl Point {
    /// Creates a new point with ID and vector.
    pub fn new(id: impl Into<PointId>, vector: Vec<f32>) -> Self {
        Self {
            id: id.into(),
            vector,
            payload: None,
        }
    }

    /// Attaches an arbitrary JSON metadata payload to the point.
    pub fn with_payload(mut self, payload: serde_json::Value) -> Self {
        self.payload = Some(payload);
        self
    }
}

/// A single matched result returned by a vector search.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    /// The unique identifier of the matching point.
    pub id: PointId,
    /// Distance metric value between query vector and point (lower is closer).
    pub distance: f32,
    /// Normalized or converted similarity score (higher is more similar).
    pub score: f32,
    /// The vector data, if requested by the query.
    pub vector: Option<Vec<f32>>,
    /// The associated metadata payload, if requested by the query.
    pub payload: Option<serde_json::Value>,
}

/// A vector search query with optional filters, ranking parameters, and response options.
#[derive(Debug, Clone)]
pub struct SearchQuery {
    pub vector: Vec<f32>,
    pub top_k: usize,
    pub ef_search: Option<usize>,
    pub filter: Option<Filter>,
    pub with_vectors: bool,
    pub with_payload: bool,
}

impl SearchQuery {
    /// Creates a new search query for the given vector and number of top results.
    pub fn new(vector: Vec<f32>, top_k: usize) -> Self {
        Self {
            vector,
            top_k,
            ef_search: None,
            filter: None,
            with_vectors: false,
            with_payload: true,
        }
    }

    /// Sets the dynamic exploration depth (beam size) for HNSW graph search.
    pub fn with_ef_search(mut self, ef: usize) -> Self {
        self.ef_search = Some(ef);
        self
    }

    /// Applies a structured metadata filter to the query.
    pub fn with_filter(mut self, filter: Filter) -> Self {
        self.filter = Some(filter);
        self
    }

    /// Sets whether to include the raw vector in the results.
    pub fn include_vector(mut self, include: bool) -> Self {
        self.with_vectors = include;
        self
    }

    /// Sets whether to include the metadata payload in the results.
    pub fn include_payload(mut self, include: bool) -> Self {
        self.with_payload = include;
        self
    }
}

/// Supported index types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum IndexType {
    /// Hierarchical Navigable Small World graph (Fast Approximate Nearest Neighbor).
    #[default]
    Hnsw,
    /// Brute-force linear scan index (100% ground truth recall).
    Flat,
}

/// Configuration parameters for HNSW index.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct HnswConfig {
    /// Max outgoing connections per node in higher layers. Typical: 16.
    pub m: usize,
    /// Max outgoing connections per node in base layer 0. Typical: 32 (2 * m).
    pub m0: usize,
    /// Dynamic candidate exploration depth during graph construction. Typical: 100 or 128.
    pub ef_construction: usize,
    /// Dynamic candidate exploration depth during search. Typical: 64.
    pub ef_search: usize,
}

impl Default for HnswConfig {
    fn default() -> Self {
        Self {
            m: 16,
            m0: 32,
            ef_construction: 100,
            ef_search: 64,
        }
    }
}

/// Configuration for a Vectrust Collection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectionConfig {
    /// Fixed dimension of vectors in this collection.
    pub dimension: usize,
    /// Distance metric to use for comparisons.
    pub metric: Metric,
    /// Type of index (HNSW or Flat).
    pub index_type: IndexType,
    /// HNSW-specific parameters (ignored if Flat index).
    pub hnsw_config: HnswConfig,
    /// Whether to force an fsync to disk on every mutating write operation.
    pub sync_on_write: bool,
}

impl CollectionConfig {
    /// Creates a builder for `CollectionConfig`.
    pub fn builder(dimension: usize, metric: Metric) -> CollectionConfigBuilder {
        CollectionConfigBuilder::new(dimension, metric)
    }
}

/// Builder for `CollectionConfig`.
pub struct CollectionConfigBuilder {
    config: CollectionConfig,
}

impl CollectionConfigBuilder {
    pub fn new(dimension: usize, metric: Metric) -> Self {
        Self {
            config: CollectionConfig {
                dimension,
                metric,
                index_type: IndexType::Hnsw,
                hnsw_config: HnswConfig::default(),
                sync_on_write: false,
            },
        }
    }

    pub fn index_type(mut self, index_type: IndexType) -> Self {
        self.config.index_type = index_type;
        self
    }

    pub fn hnsw_config(mut self, hnsw_config: HnswConfig) -> Self {
        self.config.hnsw_config = hnsw_config;
        self
    }

    pub fn sync_on_write(mut self, sync: bool) -> Self {
        self.config.sync_on_write = sync;
        self
    }

    pub fn build(self) -> CollectionConfig {
        self.config
    }
}
