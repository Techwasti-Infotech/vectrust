use std::fs::{self, OpenOptions};
use std::io::Write;
use tempfile::tempdir;
use vectrust::prelude::*;
use vectrust::storage::{load_snapshot, Wal};

#[test]
fn test_tampered_snapshot_crc32_detection() -> vectrust::Result<()> {
    let tmp = tempdir().unwrap();
    let db_path = tmp.path().to_path_buf();

    let config = CollectionConfig::builder(4, Metric::Cosine).build();

    // 1. Create collection, insert data, and create snapshot
    {
        let db = Vectrust::open(&db_path)?;
        let col = db.create_collection("audit_col", config)?;
        col.insert(Point::new(1u64, vec![0.5, 0.5, 0.5, 0.5]))?;
        col.insert(Point::new(2u64, vec![0.1, 0.9, 0.2, 0.3]))?;
        col.compact()?;
    }

    let snap_path = db_path.join("collections").join("audit_col").join("snapshot.bin");
    assert!(snap_path.exists());

    // 2. Load snapshot directly: should succeed
    let valid_data = load_snapshot(&snap_path)?;
    assert!(valid_data.is_some());

    // 3. Intentionally mutate bytes in the snapshot payload to corrupt the file
    let mut bytes = fs::read(&snap_path)?;
    assert!(bytes.len() > 16);
    // Mutate a byte in the middle of the file
    let mid = bytes.len() / 2;
    bytes[mid] ^= 0xFF;
    fs::write(&snap_path, &bytes)?;

    // 4. Verify load_snapshot now detects CRC mismatch and returns an error!
    let load_result = load_snapshot(&snap_path);
    assert!(load_result.is_err(), "Corrupted snapshot must fail CRC32 verification");
    match load_result {
        Err(VectrustError::CorruptedData(msg)) => {
            assert!(msg.contains("CRC32") || msg.contains("mismatch"));
            println!("Correctly caught CRC32 error: {}", msg);
        }
        other => panic!("Expected Err(CorruptedData), got {:?}", other.is_err()),
    }

    Ok(())
}

#[test]
fn test_corrupted_wal_trailing_record() -> vectrust::Result<()> {
    let tmp = tempdir().unwrap();
    let wal_path = tmp.path().join("test.wal");

    // 1. Write 2 valid WAL records
    {
        let mut wal = Wal::open(&wal_path, false)?;
        wal.append(&vectrust::storage::WalRecord::Insert(
            (&Point::new(10u64, vec![1.0, 2.0])).into(),
        ))?;
        wal.append(&vectrust::storage::WalRecord::Insert(
            (&Point::new(20u64, vec![3.0, 4.0])).into(),
        ))?;
    }

    // Verify 2 records can be read
    let records = Wal::read_all(&wal_path)?;
    assert_eq!(records.len(), 2);

    // 2. Append garbage/truncated bytes to the end of the WAL file
    {
        let mut file = OpenOptions::new().append(true).open(&wal_path)?;
        file.write_all(b"TRUNCATED_CORRUPT_BYTES_WITHOUT_VALID_CRC")?;
    }

    // 3. Attempting to read records: WAL safely stops at corruption boundary and recovers valid records
    let records_after_crash = Wal::read_all(&wal_path)?;
    assert_eq!(records_after_crash.len(), 2, "WAL must recover all committed records prior to the crash");

    Ok(())
}
