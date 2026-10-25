pub mod flat;
pub mod hnsw;

use std::cmp::Ordering;
use crate::error::Result;
use crate::metric::Metric;
use crate::types::{Point, PointId, SearchQuery, SearchResult};

/// Helper struct for distance priority queues (Min/Max heaps).
#[derive(Clone, Copy, PartialEq)]
pub struct DistCandidate {
    pub idx: usize,
    pub dist: f32,
}

impl Eq for DistCandidate {}

impl PartialOrd for DistCandidate {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for DistCandidate {
    fn cmp(&self, other: &Self) -> Ordering {
        self.dist.total_cmp(&other.dist)
    }
}

/// Common trait implemented by all vector indexing structures in Vectrust.
pub trait VectorIndex: Send + Sync {
    /// Inserts a new point into the index.
    fn insert(&mut self, point: &Point) -> Result<()>;

    /// Inserts a batch of points, utilizing parallel graph construction if supported.
    fn insert_batch(&mut self, points: &[Point]) -> Result<()>;

    /// Deletes a point by its identifier. Returns true if removed, false if not found.
    fn delete(&mut self, id: &PointId) -> Result<bool>;

    /// Executes a k-nearest neighbor search, respecting optional filter predicates.
    fn search(
        &self,
        query: &SearchQuery,
        metric: Metric,
        filter_fn: Option<&(dyn Fn(&PointId) -> bool + Sync + Send)>,
    ) -> Result<Vec<SearchResult>>;

    /// Total number of active vectors indexed.
    fn len(&self) -> usize;

    /// Checks if the index is empty.
    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Checks if the given point ID is present.
    fn contains(&self, id: &PointId) -> bool;

    /// Retrieves raw vector slice by point ID if present.
    fn get_vector(&self, id: &PointId) -> Option<&[f32]>;

    /// Returns all active point IDs in the index.
    fn all_ids(&self) -> Vec<PointId>;
}
