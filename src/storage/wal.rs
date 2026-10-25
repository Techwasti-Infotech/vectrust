use std::fs::{File, OpenOptions};
use std::io::{BufReader, BufWriter, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::types::{Point, PointId};

const WAL_MAGIC: &[u8; 4] = b"WAL1";

/// A serializable vector point representation for persistent storage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializedPoint {
    pub id: PointId,
    pub vector: Vec<f32>,
    pub payload: Option<String>,
}

impl From<&Point> for SerializedPoint {
    fn from(p: &Point) -> Self {
        Self {
            id: p.id.clone(),
            vector: p.vector.clone(),
            payload: p.payload.as_ref().map(|v| v.to_string()),
        }
    }
}

impl From<SerializedPoint> for Point {
    fn from(sp: SerializedPoint) -> Self {
        Self {
            id: sp.id,
            vector: sp.vector,
            payload: sp.payload.and_then(|s| serde_json::from_str(&s).ok()),
        }
    }
}

/// A mutation event recorded in the Write-Ahead Log.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WalRecord {
    Insert(SerializedPoint),
    Delete(PointId),
    UpdatePayload {
        id: PointId,
        payload: Option<String>,
    },
}

/// Append-only Write-Ahead Log for crash resilience and durability.
pub struct Wal {
    path: PathBuf,
    writer: BufWriter<File>,
    sync_on_write: bool,
}

impl Wal {
    /// Opens or creates a WAL file at the given path.
    pub fn open(path: impl AsRef<Path>, sync_on_write: bool) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        let exists = path.exists();

        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(&path)?;

        let is_empty = !exists || file.metadata()?.len() == 0;
        if is_empty {
            file.write_all(WAL_MAGIC)?;
            file.flush()?;
        } else {
            file.seek(SeekFrom::End(0))?;
        }

        let wal = Self {
            path,
            writer: BufWriter::new(file),
            sync_on_write,
        };

        Ok(wal)
    }

    /// Returns the path to the WAL file.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Appends a new record to the WAL.
    pub fn append(&mut self, record: &WalRecord) -> Result<()> {
        let data = bincode::serialize(record)?;
        let len = data.len() as u32;
        let crc = crc32fast::hash(&data);

        self.writer.write_all(&len.to_le_bytes())?;
        self.writer.write_all(&crc.to_le_bytes())?;
        self.writer.write_all(&data)?;
        self.writer.flush()?;

        if self.sync_on_write {
            self.writer.get_ref().sync_all()?;
        }

        Ok(())
    }

    /// Reads and returns all valid records from the WAL.
    pub fn read_all(path: impl AsRef<Path>) -> Result<Vec<WalRecord>> {
        let path = path.as_ref();
        if !path.exists() {
            return Ok(Vec::new());
        }

        let file = File::open(path)?;
        let mut reader = BufReader::new(file);

        let mut magic = [0u8; 4];
        if reader.read_exact(&mut magic).is_err() || &magic != WAL_MAGIC {
            // Not a valid WAL or empty
            return Ok(Vec::new());
        }

        let mut records = Vec::new();
        let mut len_buf = [0u8; 4];
        let mut crc_buf = [0u8; 4];

        loop {
            // Read length
            if reader.read_exact(&mut len_buf).is_err() {
                break; // EOF
            }
            let len = u32::from_le_bytes(len_buf) as usize;

            // Read CRC
            if reader.read_exact(&mut crc_buf).is_err() {
                break; // Incomplete record at EOF
            }
            let expected_crc = u32::from_le_bytes(crc_buf);

            // Read data payload
            let mut data = vec![0u8; len];
            if reader.read_exact(&mut data).is_err() {
                break; // Corrupted / truncated at EOF
            }

            let actual_crc = crc32fast::hash(&data);
            if actual_crc != expected_crc {
                // CRC mismatch indicates crash during write; stop replaying here
                break;
            }

            match bincode::deserialize::<WalRecord>(&data) {
                Ok(rec) => records.push(rec),
                Err(_) => break,
            }
        }

        Ok(records)
    }

    /// Truncates the WAL file, resetting it to only contain the header magic.
    pub fn truncate(&mut self) -> Result<()> {
        self.writer.flush()?;
        let file = self.writer.get_mut();
        file.set_len(0)?;
        file.seek(SeekFrom::Start(0))?;
        file.write_all(WAL_MAGIC)?;
        file.flush()?;
        if self.sync_on_write {
            file.sync_all()?;
        }
        Ok(())
    }
}
