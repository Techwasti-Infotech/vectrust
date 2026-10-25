use std::collections::{BinaryHeap, HashMap};
use serde::{Deserialize, Serialize};
use rayon::prelude::*;

use crate::error::{Result, VectrustError};
use crate::index::{DistCandidate, VectorIndex};
use crate::metric::Metric;
use crate::types::{Point, PointId, SearchQuery, SearchResult};

/// Flat (brute-force) exact vector index.
/// Provides 100% ground-truth recall and serves as the baseline for accuracy.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlatIndex {
    pub dimension: usize,
    pub ids: Vec<PointId>,
    pub vectors: Vec<f32>,
    #[serde(skip)]
    pub id_to_idx: HashMap<PointId, usize>,
}

impl FlatIndex {
    pub fn new(dimension: usize) -> Self {
        Self {
            dimension,
            ids: Vec::new(),
            vectors: Vec::new(),
            id_to_idx: HashMap::new(),
        }
    }

    /// Rebuilds the fast lookup map after deserialization.
    pub fn rebuild_map(&mut self) {
        self.id_to_idx.clear();
        for (i, id) in self.ids.iter().enumerate() {
            self.id_to_idx.insert(id.clone(), i);
        }
    }

    #[inline]
    fn get_vector_slice(&self, idx: usize) -> &[f32] {
        let start = idx * self.dimension;
        &self.vectors[start..start + self.dimension]
    }
}

impl VectorIndex for FlatIndex {
    fn insert(&mut self, point: &Point) -> Result<()> {
        if point.vector.len() != self.dimension {
            return Err(VectrustError::DimensionMismatch {
                expected: self.dimension,
                got: point.vector.len(),
            });
        }

        // If exists, overwrite vector
        if let Some(&idx) = self.id_to_idx.get(&point.id) {
            let start = idx * self.dimension;
            self.vectors[start..start + self.dimension].copy_from_slice(&point.vector);
            return Ok(());
        }

        let new_idx = self.ids.len();
        self.ids.push(point.id.clone());
        self.vectors.extend_from_slice(&point.vector);
        self.id_to_idx.insert(point.id.clone(), new_idx);
        Ok(())
    }

    fn insert_batch(&mut self, points: &[Point]) -> Result<()> {
        for point in points {
            self.insert(point)?;
        }
        Ok(())
    }

    fn delete(&mut self, id: &PointId) -> Result<bool> {
        let Some(&idx) = self.id_to_idx.get(id) else {
            return Ok(false);
        };

        let last_idx = self.ids.len() - 1;
        if idx != last_idx {
            // Swap remove vector
            let start = idx * self.dimension;
            let last_start = last_idx * self.dimension;
            for i in 0..self.dimension {
                self.vectors[start + i] = self.vectors[last_start + i];
            }
            // Swap remove ID
            self.ids[idx] = self.ids[last_idx].clone();
            self.id_to_idx.insert(self.ids[idx].clone(), idx);
        }

        self.ids.pop();
        self.vectors.truncate(self.ids.len() * self.dimension);
        self.id_to_idx.remove(id);
        Ok(true)
    }

    fn search(
        &self,
        query: &SearchQuery,
        metric: Metric,
        filter_fn: Option<&(dyn Fn(&PointId) -> bool + Sync + Send)>,
    ) -> Result<Vec<SearchResult>> {
        if query.vector.len() != self.dimension {
            return Err(VectrustError::DimensionMismatch {
                expected: self.dimension,
                got: query.vector.len(),
            });
        }

        let top_k = query.top_k;
        if top_k == 0 || self.ids.is_empty() {
            return Ok(Vec::new());
        }

        // For large datasets, compute in parallel using Rayon; otherwise sequential.
        let threshold_for_parallel = 2000;
        let candidates: Vec<DistCandidate> = if self.ids.len() >= threshold_for_parallel {
            (0..self.ids.len())
                .into_par_iter()
                .filter(|&idx| {
                    if let Some(f) = filter_fn {
                        f(&self.ids[idx])
                    } else {
                        true
                    }
                })
                .map(|idx| {
                    let vec_slice = self.get_vector_slice(idx);
                    let dist = metric.distance(&query.vector, vec_slice).unwrap_or(f32::INFINITY);
                    DistCandidate { idx, dist }
                })
                .collect()
        } else {
            let mut matches = Vec::with_capacity(self.ids.len());
            for idx in 0..self.ids.len() {
                if let Some(f) = filter_fn {
                    if !f(&self.ids[idx]) {
                        continue;
                    }
                }
                let vec_slice = self.get_vector_slice(idx);
                let dist = metric.distance(&query.vector, vec_slice)?;
                matches.push(DistCandidate { idx, dist });
            }
            matches
        };

        // Maintain top_k closest (min distance) using a max-heap of size top_k
        let mut heap: BinaryHeap<DistCandidate> = BinaryHeap::with_capacity(top_k);
        for candidate in candidates {
            if heap.len() < top_k {
                heap.push(candidate);
            } else if let Some(worst) = heap.peek() {
                if candidate.dist < worst.dist {
                    heap.pop();
                    heap.push(candidate);
                }
            }
        }

        let mut sorted = heap.into_sorted_vec(); // Ascending order
        let mut results = Vec::with_capacity(sorted.len());
        for c in sorted.drain(..) {
            let id = self.ids[c.idx].clone();
            let score = metric.similarity(&query.vector, self.get_vector_slice(c.idx))?;
            let vector = if query.with_vectors {
                Some(self.get_vector_slice(c.idx).to_vec())
            } else {
                None
            };
            results.push(SearchResult {
                id,
                distance: c.dist,
                score,
                vector,
                payload: None, // Filled by collection layer
            });
        }

        Ok(results)
    }

    fn len(&self) -> usize {
        self.ids.len()
    }

    fn contains(&self, id: &PointId) -> bool {
        self.id_to_idx.contains_key(id)
    }

    fn get_vector(&self, id: &PointId) -> Option<&[f32]> {
        let &idx = self.id_to_idx.get(id)?;
        Some(self.get_vector_slice(idx))
    }

    fn all_ids(&self) -> Vec<PointId> {
        self.ids.clone()
    }
}
