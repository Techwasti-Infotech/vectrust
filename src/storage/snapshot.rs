use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::path::Path;
use serde::{Deserialize, Serialize};

use crate::error::{Result, VectrustError};
use crate::index::flat::FlatIndex;
use crate::index::hnsw::HnswIndex;
use crate::types::{CollectionConfig, PointId};

const SNAPSHOT_MAGIC: &[u8; 4] = b"VECS";
const SNAPSHOT_VERSION: u16 = 1;

/// Payload holding the entire serializable state of a collection.
#[derive(Serialize, Deserialize)]
pub struct SnapshotData {
    pub config: CollectionConfig,
    pub flat_index: Option<FlatIndex>,
    pub hnsw_index: Option<HnswIndex>,
    pub payloads: HashMap<PointId, String>,
}

/// Atomically saves snapshot data to disk with CRC32 verification.
pub fn save_snapshot(path: impl AsRef<Path>, data: &SnapshotData) -> Result<()> {
    let path = path.as_ref();
    let tmp_path = path.with_extension("tmp");

    let payload_bytes = bincode::serialize(data)?;
    let crc = crc32fast::hash(&payload_bytes);
    let len = payload_bytes.len() as u64;

    {
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&tmp_path)?;

        file.write_all(SNAPSHOT_MAGIC)?;
        file.write_all(&SNAPSHOT_VERSION.to_le_bytes())?;
        file.write_all(&crc.to_le_bytes())?;
        file.write_all(&len.to_le_bytes())?;
        file.write_all(&payload_bytes)?;
        file.flush()?;
        file.sync_all()?;
    }

    if path.exists() {
        let _ = std::fs::remove_file(path);
    }

    std::fs::rename(&tmp_path, path)?;

    Ok(())
}

/// Loads and validates a snapshot from disk.
pub fn load_snapshot(path: impl AsRef<Path>) -> Result<Option<SnapshotData>> {
    let path = path.as_ref();
    if !path.exists() {
        return Ok(None);
    }

    let mut file = File::open(path)?;

    let mut magic = [0u8; 4];
    file.read_exact(&mut magic)?;
    if &magic != SNAPSHOT_MAGIC {
        return Err(VectrustError::CorruptedData(
            "Invalid snapshot magic header".into(),
        ));
    }

    let mut version_buf = [0u8; 2];
    file.read_exact(&mut version_buf)?;
    let version = u16::from_le_bytes(version_buf);
    if version != SNAPSHOT_VERSION {
        return Err(VectrustError::CorruptedData(format!(
            "Unsupported snapshot version: {}",
            version
        )));
    }

    let mut crc_buf = [0u8; 4];
    file.read_exact(&mut crc_buf)?;
    let expected_crc = u32::from_le_bytes(crc_buf);

    let mut len_buf = [0u8; 8];
    file.read_exact(&mut len_buf)?;
    let len = u64::from_le_bytes(len_buf) as usize;

    let mut payload_bytes = vec![0u8; len];
    file.read_exact(&mut payload_bytes)?;

    let actual_crc = crc32fast::hash(&payload_bytes);
    if actual_crc != expected_crc {
        return Err(VectrustError::CorruptedData(format!(
            "CRC32 checksum mismatch: expected {}, got {}",
            expected_crc, actual_crc
        )));
    }

    let mut data: SnapshotData = bincode::deserialize(&payload_bytes)?;

    // Rebuild in-memory acceleration lookup tables
    if let Some(flat) = &mut data.flat_index {
        flat.rebuild_map();
    }
    if let Some(hnsw) = &mut data.hnsw_index {
        hnsw.rebuild_map();
    }

    Ok(Some(data))
}
