use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use parking_lot::RwLock;

use crate::collection::Collection;
use crate::error::{Result, VectrustError};
use crate::types::CollectionConfig;

/// The primary Vectrust embedded database engine.
/// Coordinates multiple named vector collections, storage directories, and lifecycle management.
#[derive(Clone)]
pub struct Vectrust {
    storage_dir: Option<PathBuf>,
    collections: Arc<RwLock<HashMap<String, Collection>>>,
}

impl Vectrust {
    /// Opens or creates an embedded Vectrust database stored at the specified path.
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let dir = path.as_ref().to_path_buf();
        let collections_dir = dir.join("collections");
        fs::create_dir_all(&collections_dir)?;

        let db = Self {
            storage_dir: Some(dir),
            collections: Arc::new(RwLock::new(HashMap::new())),
        };

        // Scan existing collections on disk
        if let Ok(entries) = fs::read_dir(&collections_dir) {
            for entry in entries.flatten() {
                if entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false) {
                    let col_name = entry.file_name().to_string_lossy().to_string();
                    let col_path = entry.path();
                    let snapshot_path = col_path.join("snapshot.bin");

                    if snapshot_path.exists() {
                        if let Ok(Some(snap)) = crate::storage::load_snapshot(&snapshot_path) {
                            if let Ok(col) = Collection::open(&col_path, snap.config) {
                                db.collections.write().insert(col_name, col);
                            }
                        }
                    }
                }
            }
        }

        Ok(db)
    }

    /// Creates an ephemeral, in-memory Vectrust database engine.
    pub fn in_memory() -> Self {
        Self {
            storage_dir: None,
            collections: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Creates a new named collection. Returns an error if it already exists.
    pub fn create_collection(&self, name: &str, config: CollectionConfig) -> Result<Collection> {
        let mut collections = self.collections.write();
        if collections.contains_key(name) {
            return Err(VectrustError::CollectionAlreadyExists(name.to_string()));
        }

        let collection = match &self.storage_dir {
            Some(dir) => {
                let col_path = dir.join("collections").join(name);
                Collection::open(col_path, config)?
            }
            None => Collection::in_memory(config),
        };

        collections.insert(name.to_string(), collection.clone());
        Ok(collection)
    }

    /// Retrieves an existing collection by name.
    pub fn get_collection(&self, name: &str) -> Option<Collection> {
        let collections = self.collections.read();
        collections.get(name).cloned()
    }

    /// Gets an existing collection or creates a new one with the given configuration if missing.
    pub fn get_or_create_collection(
        &self,
        name: &str,
        config: CollectionConfig,
    ) -> Result<Collection> {
        let mut collections = self.collections.write();
        if let Some(col) = collections.get(name) {
            return Ok(col.clone());
        }

        let collection = match &self.storage_dir {
            Some(dir) => {
                let col_path = dir.join("collections").join(name);
                Collection::open(col_path, config)?
            }
            None => Collection::in_memory(config),
        };

        collections.insert(name.to_string(), collection.clone());
        Ok(collection)
    }

    /// Drops a collection and removes any persistent files from disk.
    pub fn drop_collection(&self, name: &str) -> Result<bool> {
        let mut collections = self.collections.write();
        let removed = collections.remove(name);

        if let Some(storage_dir) = &self.storage_dir {
            let col_path = storage_dir.join("collections").join(name);
            if col_path.exists() {
                fs::remove_dir_all(col_path)?;
            }
        }

        Ok(removed.is_some())
    }

    /// Returns a list of all active collection names.
    pub fn list_collections(&self) -> Vec<String> {
        let collections = self.collections.read();
        collections.keys().cloned().collect()
    }

    /// Flushes and compacts all open collections.
    pub fn flush_all(&self) -> Result<()> {
        let collections = self.collections.read();
        for col in collections.values() {
            col.flush()?;
        }
        Ok(())
    }
}
