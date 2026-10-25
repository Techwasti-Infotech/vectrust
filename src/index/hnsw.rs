use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap, HashSet};
use rand::Rng;
use serde::{Deserialize, Serialize};

use crate::error::{Result, VectrustError};
use crate::index::{DistCandidate, VectorIndex};
use crate::metric::Metric;
use crate::types::{HnswConfig, Point, PointId, SearchQuery, SearchResult};

/// Internal node representation in the HNSW graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HnswNode {
    pub id: PointId,
    pub vector: Vec<f32>,
    pub level: usize,
    /// Adjacency list per layer: `neighbors[layer]` contains internal indices of neighbors.
    pub neighbors: Vec<Vec<usize>>,
}

/// Hierarchical Navigable Small World (HNSW) vector index.
/// Delivers logarithmic search complexity and high recall for large vector collections.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HnswIndex {
    pub dimension: usize,
    pub metric: Metric,
    pub config: HnswConfig,
    pub nodes: Vec<Option<HnswNode>>,
    pub entry_point: Option<usize>,
    pub max_level: usize,
    pub active_count: usize,
    #[serde(skip)]
    pub id_to_internal: HashMap<PointId, usize>,
    #[serde(skip)]
    ml: f64,
}

impl HnswIndex {
    /// Creates a new empty HNSW index.
    pub fn new(dimension: usize, metric: Metric, config: HnswConfig) -> Self {
        let m = config.m.max(2);
        let ml = 1.0 / (m as f64).ln();
        Self {
            dimension,
            metric,
            config,
            nodes: Vec::new(),
            entry_point: None,
            max_level: 0,
            active_count: 0,
            id_to_internal: HashMap::new(),
            ml,
        }
    }

    /// Rebuilds internal lookup mappings after deserialization.
    pub fn rebuild_map(&mut self) {
        self.id_to_internal.clear();
        let m = self.config.m.max(2);
        self.ml = 1.0 / (m as f64).ln();
        self.active_count = 0;
        for (idx, node_opt) in self.nodes.iter().enumerate() {
            if let Some(node) = node_opt {
                self.id_to_internal.insert(node.id.clone(), idx);
                self.active_count += 1;
            }
        }
    }

    /// Random level generation based on geometric distribution.
    fn random_level(&self) -> usize {
        let mut rng = rand::thread_rng();
        let r: f64 = rng.gen_range(1e-9..1.0);
        let level = (-r.ln() * self.ml).floor() as usize;
        level.min(16) // Safety cap
    }

    #[inline]
    fn dist(&self, a: &[f32], b: &[f32]) -> f32 {
        self.metric.distance(a, b).unwrap_or(f32::INFINITY)
    }

    #[inline]
    fn node_vector(&self, idx: usize) -> &[f32] {
        &self.nodes[idx].as_ref().unwrap().vector
    }

    /// Greedy search in layer `level` for the single closest node to `query`.
    fn search_layer_greedy(&self, query: &[f32], mut curr_obj: usize, level: usize) -> usize {
        let mut curr_dist = self.dist(query, self.node_vector(curr_obj));

        loop {
            let mut changed = false;
            let neighbors = &self.nodes[curr_obj].as_ref().unwrap().neighbors[level];

            for &neighbor in neighbors {
                if let Some(n_node) = &self.nodes[neighbor] {
                    let d = self.dist(query, &n_node.vector);
                    if d < curr_dist {
                        curr_dist = d;
                        curr_obj = neighbor;
                        changed = true;
                    }
                }
            }

            if !changed {
                break;
            }
        }

        curr_obj
    }

    /// Explores layer `level` using dynamic candidate priority queue of size `ef`.
    fn search_layer(
        &self,
        query: &[f32],
        entry: usize,
        ef: usize,
        level: usize,
    ) -> Vec<DistCandidate> {
        let mut visited = HashSet::new();
        visited.insert(entry);

        let initial_dist = self.dist(query, self.node_vector(entry));

        // Min-heap for candidates to explore: closest first
        let mut candidates = BinaryHeap::new();
        candidates.push(Reverse(DistCandidate {
            idx: entry,
            dist: initial_dist,
        }));

        // Max-heap for dynamic list of nearest elements found
        let mut best = BinaryHeap::new();
        best.push(DistCandidate {
            idx: entry,
            dist: initial_dist,
        });

        while let Some(Reverse(curr)) = candidates.pop() {
            let worst_dist = best.peek().map(|c| c.dist).unwrap_or(f32::INFINITY);
            if curr.dist > worst_dist {
                break;
            }

            let neighbors = &self.nodes[curr.idx].as_ref().unwrap().neighbors[level];
            for &neighbor in neighbors {
                if visited.insert(neighbor) {
                    if let Some(n_node) = &self.nodes[neighbor] {
                        let d = self.dist(query, &n_node.vector);
                        let worst_dist = best.peek().map(|c| c.dist).unwrap_or(f32::INFINITY);

                        if d < worst_dist || best.len() < ef {
                            candidates.push(Reverse(DistCandidate {
                                idx: neighbor,
                                dist: d,
                            }));
                            best.push(DistCandidate {
                                idx: neighbor,
                                dist: d,
                            });

                            if best.len() > ef {
                                best.pop();
                            }
                        }
                    }
                }
            }
        }

        best.into_sorted_vec() // Sorted ascending by distance
    }

    /// Selects neighbors for edge connection, enforcing max degree `m_max`.
    fn select_neighbors(
        &self,
        candidates: &[DistCandidate],
        m_max: usize,
    ) -> Vec<usize> {
        candidates.iter().take(m_max).map(|c| c.idx).collect()
    }

    /// Connects two nodes bidirectionally at layer `level`, maintaining degree bound `m_max`.
    fn connect_bidirectional(
        &mut self,
        src: usize,
        dst: usize,
        level: usize,
        m_max: usize,
    ) {
        // Add dst to src
        if let Some(src_node) = &mut self.nodes[src] {
            if !src_node.neighbors[level].contains(&dst) {
                src_node.neighbors[level].push(dst);
            }
        }
        self.shrink_connections(src, level, m_max);

        // Add src to dst
        if let Some(dst_node) = &mut self.nodes[dst] {
            if !dst_node.neighbors[level].contains(&src) {
                dst_node.neighbors[level].push(src);
            }
        }
        self.shrink_connections(dst, level, m_max);
    }

    /// Shrinks the connections of `idx` at `level` if it exceeds `m_max`.
    fn shrink_connections(&mut self, idx: usize, level: usize, m_max: usize) {
        let node_vec = self.node_vector(idx).to_vec();
        let neighbors = &self.nodes[idx].as_ref().unwrap().neighbors[level];
        if neighbors.len() <= m_max {
            return;
        }

        let mut dist_list: Vec<DistCandidate> = neighbors
            .iter()
            .filter_map(|&n| {
                self.nodes[n].as_ref().map(|n_node| DistCandidate {
                    idx: n,
                    dist: self.dist(&node_vec, &n_node.vector),
                })
            })
            .collect();

        dist_list.sort_unstable();
        dist_list.truncate(m_max);

        let new_neighbors: Vec<usize> = dist_list.into_iter().map(|c| c.idx).collect();
        self.nodes[idx].as_mut().unwrap().neighbors[level] = new_neighbors;
    }
}

impl VectorIndex for HnswIndex {
    fn insert(&mut self, point: &Point) -> Result<()> {
        if point.vector.len() != self.dimension {
            return Err(VectrustError::DimensionMismatch {
                expected: self.dimension,
                got: point.vector.len(),
            });
        }

        // Check if ID already exists; if so, delete it first (upsert)
        if self.id_to_internal.contains_key(&point.id) {
            self.delete(&point.id)?;
        }

        let node_level = self.random_level();
        let new_idx = self.nodes.len();

        let new_node = HnswNode {
            id: point.id.clone(),
            vector: point.vector.clone(),
            level: node_level,
            neighbors: vec![Vec::new(); node_level + 1],
        };

        self.nodes.push(Some(new_node));
        self.id_to_internal.insert(point.id.clone(), new_idx);
        self.active_count += 1;

        let Some(mut curr_obj) = self.entry_point else {
            self.entry_point = Some(new_idx);
            self.max_level = node_level;
            return Ok(());
        };

        let mut curr_level = self.max_level;

        // Step 1: Greedy top-down search from max_level to node_level + 1
        while curr_level > node_level {
            curr_obj = self.search_layer_greedy(&point.vector, curr_obj, curr_level);
            curr_level -= 1;
        }

        // Step 2: Layer search and link addition from min(node_level, max_level) down to 0
        let bottom_level = node_level.min(self.max_level);
        for l in (0..=bottom_level).rev() {
            let m_max = if l == 0 { self.config.m0 } else { self.config.m };
            let candidates = self.search_layer(&point.vector, curr_obj, self.config.ef_construction, l);
            let neighbors = self.select_neighbors(&candidates, m_max);

            for &neighbor in &neighbors {
                self.connect_bidirectional(new_idx, neighbor, l, m_max);
            }

            if let Some(closest) = candidates.first() {
                curr_obj = closest.idx;
            }
        }

        // Step 3: Update entry point if newly added node has higher level
        if node_level > self.max_level {
            self.max_level = node_level;
            self.entry_point = Some(new_idx);
        }

        Ok(())
    }

    fn insert_batch(&mut self, points: &[Point]) -> Result<()> {
        for point in points {
            self.insert(point)?;
        }
        Ok(())
    }

    fn delete(&mut self, id: &PointId) -> Result<bool> {
        let Some(&idx) = self.id_to_internal.get(id) else {
            return Ok(false);
        };

        self.id_to_internal.remove(id);
        self.nodes[idx] = None;
        self.active_count = self.active_count.saturating_sub(1);

        // If the entry point was deleted, locate a new entry point
        if self.entry_point == Some(idx) {
            let mut highest_level = 0;
            let mut new_entry = None;
            for (i, node_opt) in self.nodes.iter().enumerate() {
                if let Some(node) = node_opt {
                    if node.level >= highest_level {
                        highest_level = node.level;
                        new_entry = Some(i);
                    }
                }
            }
            self.entry_point = new_entry;
            self.max_level = highest_level;
        }

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

        if self.active_count == 0 || query.top_k == 0 {
            return Ok(Vec::new());
        }

        let Some(entry) = self.entry_point else {
            return Ok(Vec::new());
        };

        let mut curr_obj = entry;

        // Step 1: Greedy top-down search to base layer 1
        for l in (1..=self.max_level).rev() {
            curr_obj = self.search_layer_greedy(&query.vector, curr_obj, l);
        }

        // Step 2: Layer 0 exploration with dynamic candidate set
        let ef = query.ef_search.unwrap_or(self.config.ef_search).max(query.top_k);
        let raw_candidates = self.search_layer(&query.vector, curr_obj, ef, 0);

        // Filter and collect top-k results
        let mut results = Vec::with_capacity(query.top_k);
        for c in raw_candidates {
            if let Some(node) = &self.nodes[c.idx] {
                if let Some(filter) = filter_fn {
                    if !filter(&node.id) {
                        continue;
                    }
                }

                let score = metric.similarity(&query.vector, &node.vector)?;
                let vector = if query.with_vectors {
                    Some(node.vector.clone())
                } else {
                    None
                };

                results.push(SearchResult {
                    id: node.id.clone(),
                    distance: c.dist,
                    score,
                    vector,
                    payload: None,
                });

                if results.len() >= query.top_k {
                    break;
                }
            }
        }

        // If top-k was not satisfied because filter was very selective, perform exhaustive fallback
        if results.len() < query.top_k && filter_fn.is_some() {
            let mut remaining_candidates = Vec::new();
            for (idx, node_opt) in self.nodes.iter().enumerate() {
                if let Some(node) = node_opt {
                    if results.iter().any(|r| r.id == node.id) {
                        continue;
                    }
                    if let Some(filter) = filter_fn {
                        if !filter(&node.id) {
                            continue;
                        }
                    }
                    let dist = self.dist(&query.vector, &node.vector);
                    remaining_candidates.push(DistCandidate { idx, dist });
                }
            }

            remaining_candidates.sort_unstable();
            for c in remaining_candidates {
                if let Some(node) = &self.nodes[c.idx] {
                    let score = metric.similarity(&query.vector, &node.vector)?;
                    let vector = if query.with_vectors {
                        Some(node.vector.clone())
                    } else {
                        None
                    };
                    results.push(SearchResult {
                        id: node.id.clone(),
                        distance: c.dist,
                        score,
                        vector,
                        payload: None,
                    });
                    if results.len() >= query.top_k {
                        break;
                    }
                }
            }
        }

        Ok(results)
    }

    fn len(&self) -> usize {
        self.active_count
    }

    fn contains(&self, id: &PointId) -> bool {
        self.id_to_internal.contains_key(id)
    }

    fn get_vector(&self, id: &PointId) -> Option<&[f32]> {
        let &idx = self.id_to_internal.get(id)?;
        self.nodes[idx].as_ref().map(|n| n.vector.as_slice())
    }

    fn all_ids(&self) -> Vec<PointId> {
        self.id_to_internal.keys().cloned().collect()
    }
}
