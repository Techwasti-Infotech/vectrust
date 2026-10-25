use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use parking_lot::RwLock;
use rayon::prelude::*;

use crate::error::Result;
use crate::index::flat::FlatIndex;
use crate::index::hnsw::HnswIndex;
use crate::index::VectorIndex;
use crate::metric::Metric;
use crate::storage::{load_snapshot, save_snapshot, SnapshotData, Wal, WalRecord};
use crate::types::{CollectionConfig, IndexType, Point, PointId, SearchQuery, SearchResult};

/// Concrete storage engine supporting both HNSW and Flat indices.
pub enum IndexEngine {
    Hnsw(HnswIndex),
    Flat(FlatIndex),
}

impl VectorIndex for IndexEngine {
    fn insert(&mut self, point: &Point) -> Result<()> {
        match self {
            IndexEngine::Hnsw(idx) => idx.insert(point),
            IndexEngine::Flat(idx) => idx.insert(point),
        }
    }

    fn insert_batch(&mut self, points: &[Point]) -> Result<()> {
        match self {
            IndexEngine::Hnsw(idx) => idx.insert_batch(points),
            IndexEngine::Flat(idx) => idx.insert_batch(points),
        }
    }

    fn delete(&mut self, id: &PointId) -> Result<bool> {
        match self {
            IndexEngine::Hnsw(idx) => idx.delete(id),
            IndexEngine::Flat(idx) => idx.delete(id),
        }
    }

    fn search(
        &self,
        query: &SearchQuery,
        metric: Metric,
        filter_fn: Option<&(dyn Fn(&PointId) -> bool + Sync + Send)>,
    ) -> Result<Vec<SearchResult>> {
        match self {
            IndexEngine::Hnsw(idx) => idx.search(query, metric, filter_fn),
            IndexEngine::Flat(idx) => idx.search(query, metric, filter_fn),
        }
    }

    fn len(&self) -> usize {
        match self {
            IndexEngine::Hnsw(idx) => idx.len(),
            IndexEngine::Flat(idx) => idx.len(),
        }
    }

    fn contains(&self, id: &PointId) -> bool {
        match self {
            IndexEngine::Hnsw(idx) => idx.contains(id),
            IndexEngine::Flat(idx) => idx.contains(id),
        }
    }

    fn get_vector(&self, id: &PointId) -> Option<&[f32]> {
        match self {
            IndexEngine::Hnsw(idx) => idx.get_vector(id),
            IndexEngine::Flat(idx) => idx.get_vector(id),
        }
    }

    fn all_ids(&self) -> Vec<PointId> {
        match self {
            IndexEngine::Hnsw(idx) => idx.all_ids(),
            IndexEngine::Flat(idx) => idx.all_ids(),
        }
    }
}

struct CollectionInner {
    config: CollectionConfig,
    index: IndexEngine,
    payloads: HashMap<PointId, serde_json::Value>,
    wal: Option<Wal>,
    storage_dir: Option<PathBuf>,
}

impl CollectionInner {
    fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>> {
        let filter_fn: Option<Box<dyn Fn(&PointId) -> bool + Sync + Send>> = query.filter.as_ref().map(|f| {
            let filter = f.clone();
            let func: Box<dyn Fn(&PointId) -> bool + Sync + Send> = Box::new(move |id: &PointId| {
                let payload = self.payloads.get(id);
                filter.matches(payload)
            });
            func
        });

        let mut results = self.index.search(
            query,
            self.config.metric,
            filter_fn.as_deref(),
        )?;

        if query.with_payload {
            for res in &mut results {
                res.payload = self.payloads.get(&res.id).cloned();
            }
        }

        Ok(results)
    }
}

/// A thread-safe, concurrent vector collection.
/// Supports both in-memory and persistent durable modes.
#[derive(Clone)]
pub struct Collection {
    inner: Arc<RwLock<CollectionInner>>,
}

impl Collection {
    /// Creates a new in-memory collection.
    pub fn in_memory(config: CollectionConfig) -> Self {
        let index = match config.index_type {
            IndexType::Hnsw => IndexEngine::Hnsw(HnswIndex::new(
                config.dimension,
                config.metric,
                config.hnsw_config,
            )),
            IndexType::Flat => IndexEngine::Flat(FlatIndex::new(config.dimension)),
        };

        Self {
            inner: Arc::new(RwLock::new(CollectionInner {
                config,
                index,
                payloads: HashMap::new(),
                wal: None,
                storage_dir: None,
            })),
        }
    }

    /// Opens or creates a persistent collection on disk.
    pub fn open(storage_dir: impl AsRef<Path>, config: CollectionConfig) -> Result<Self> {
        let dir = storage_dir.as_ref().to_path_buf();
        fs::create_dir_all(&dir)?;

        let snapshot_file = dir.join("snapshot.bin");
        let wal_file = dir.join("wal.log");

        // 1. Try to load snapshot
        let mut loaded_data = load_snapshot(&snapshot_file)?;

        let (mut index, mut payloads) = if let Some(data) = loaded_data.take() {
            let idx = match data.config.index_type {
                IndexType::Hnsw => {
                    IndexEngine::Hnsw(data.hnsw_index.unwrap_or_else(|| {
                        HnswIndex::new(data.config.dimension, data.config.metric, data.config.hnsw_config)
                    }))
                }
                IndexType::Flat => {
                    IndexEngine::Flat(data.flat_index.unwrap_or_else(|| {
                        FlatIndex::new(data.config.dimension)
                    }))
                }
            };
            let decoded_payloads = data
                .payloads
                .into_iter()
                .filter_map(|(id, s)| serde_json::from_str(&s).ok().map(|v| (id, v)))
                .collect();
            (idx, decoded_payloads)
        } else {
            let idx = match config.index_type {
                IndexType::Hnsw => IndexEngine::Hnsw(HnswIndex::new(
                    config.dimension,
                    config.metric,
                    config.hnsw_config,
                )),
                IndexType::Flat => IndexEngine::Flat(FlatIndex::new(config.dimension)),
            };
            (idx, HashMap::new())
        };

        // 2. Replay Write-Ahead Log (WAL) for updates post-snapshot
        if wal_file.exists() {
            let records = Wal::read_all(&wal_file)?;
            for record in records {
                match record {
                    WalRecord::Insert(sp) => {
                        let pt: Point = sp.into();
                        index.insert(&pt)?;
                        if let Some(p) = pt.payload {
                            payloads.insert(pt.id, p);
                        }
                    }
                    WalRecord::Delete(id) => {
                        index.delete(&id)?;
                        payloads.remove(&id);
                    }
                    WalRecord::UpdatePayload { id, payload } => {
                        if let Some(p_str) = payload {
                            if let Ok(val) = serde_json::from_str(&p_str) {
                                payloads.insert(id, val);
                            }
                        } else {
                            payloads.remove(&id);
                        }
                    }
                }
            }
        }

        // 3. Open WAL for future appends
        let wal = Wal::open(&wal_file, config.sync_on_write)?;

        Ok(Self {
            inner: Arc::new(RwLock::new(CollectionInner {
                config,
                index,
                payloads,
                wal: Some(wal),
                storage_dir: Some(dir),
            })),
        })
    }

    /// Inserts a vector point into the collection.
    pub fn insert(&self, point: Point) -> Result<()> {
        let mut inner = self.inner.write();

        if let Some(wal) = &mut inner.wal {
            wal.append(&WalRecord::Insert((&point).into()))?;
        }

        if let Some(payload) = &point.payload {
            inner.payloads.insert(point.id.clone(), payload.clone());
        }

        inner.index.insert(&point)?;
        Ok(())
    }

    /// Inserts multiple points in batch.
    pub fn insert_batch(&self, points: Vec<Point>) -> Result<()> {
        let mut inner = self.inner.write();

        for point in &points {
            if let Some(wal) = &mut inner.wal {
                wal.append(&WalRecord::Insert(point.into()))?;
            }
            if let Some(payload) = &point.payload {
                inner.payloads.insert(point.id.clone(), payload.clone());
            }
        }

        inner.index.insert_batch(&points)?;
        Ok(())
    }

    /// Inserts or updates a point.
    pub fn upsert(&self, point: Point) -> Result<()> {
        self.insert(point)
    }

    /// Deletes a point by identifier. Returns true if removed, false if not found.
    pub fn delete(&self, id: impl Into<PointId>) -> Result<bool> {
        let id = id.into();
        let mut inner = self.inner.write();

        if let Some(wal) = &mut inner.wal {
            wal.append(&WalRecord::Delete(id.clone()))?;
        }

        inner.payloads.remove(&id);
        inner.index.delete(&id)
    }

    /// Updates or removes the metadata payload for an existing point.
    pub fn update_payload(
        &self,
        id: impl Into<PointId>,
        payload: Option<serde_json::Value>,
    ) -> Result<bool> {
        let id = id.into();
        let mut inner = self.inner.write();

        if !inner.index.contains(&id) {
            return Ok(false);
        }

        if let Some(wal) = &mut inner.wal {
            wal.append(&WalRecord::UpdatePayload {
                id: id.clone(),
                payload: payload.as_ref().map(|v| v.to_string()),
            })?;
        }

        if let Some(p) = payload {
            inner.payloads.insert(id, p);
        } else {
            inner.payloads.remove(&id);
        }

        Ok(true)
    }

    /// Retrieves a point by its identifier.
    pub fn get(&self, id: impl Into<PointId>) -> Result<Option<Point>> {
        let id = id.into();
        let inner = self.inner.read();

        let Some(vec_slice) = inner.index.get_vector(&id) else {
            return Ok(None);
        };

        let vector = vec_slice.to_vec();
        let payload = inner.payloads.get(&id).cloned();

        Ok(Some(Point {
            id,
            vector,
            payload,
        }))
    }

    /// Searches the collection for nearest neighbors to the query vector.
    pub fn search(&self, query: SearchQuery) -> Result<Vec<SearchResult>> {
        let inner = self.inner.read();
        inner.search(&query)
    }

    /// Executes multiple search queries concurrently across worker threads using Rayon.
    pub fn search_batch(&self, queries: Vec<SearchQuery>) -> Result<Vec<Vec<SearchResult>>> {
        let inner = self.inner.read();
        queries
            .into_par_iter()
            .map(|q| inner.search(&q))
            .collect()
    }

    /// Returns the total number of active vectors in the collection.
    pub fn count(&self) -> usize {
        let inner = self.inner.read();
        inner.index.len()
    }

    /// Checks if the collection is empty.
    pub fn is_empty(&self) -> bool {
        self.count() == 0
    }

    /// Returns the configuration of this collection.
    pub fn config(&self) -> CollectionConfig {
        let inner = self.inner.read();
        inner.config.clone()
    }

    /// Compacts the collection state: writes a clean snapshot and truncates the WAL log.
    pub fn compact(&self) -> Result<()> {
        let mut inner = self.inner.write();

        let Some(storage_dir) = &inner.storage_dir else {
            return Ok(()); // In-memory collections don't need disk compaction
        };

        let snapshot_file = storage_dir.join("snapshot.bin");

        let (flat_index, hnsw_index) = match &inner.index {
            IndexEngine::Hnsw(idx) => (None, Some(idx.clone())),
            IndexEngine::Flat(idx) => (Some(idx.clone()), None),
        };

        let encoded_payloads = inner
            .payloads
            .iter()
            .map(|(k, v)| (k.clone(), v.to_string()))
            .collect();

        let snapshot_data = SnapshotData {
            config: inner.config.clone(),
            flat_index,
            hnsw_index,
            payloads: encoded_payloads,
        };

        save_snapshot(&snapshot_file, &snapshot_data)?;

        if let Some(wal) = &mut inner.wal {
            wal.truncate()?;
        }

        Ok(())
    }

    /// Flushes any pending data to disk.
    pub fn flush(&self) -> Result<()> {
        self.compact()
    }
}
