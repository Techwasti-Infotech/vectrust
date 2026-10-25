use std::process::Command;
use tempfile::tempdir;
use vectrust::prelude::*;

#[test]
fn test_cli_binary_execution() -> vectrust::Result<()> {
    // 1. Prepare a temporary database on disk
    let tmp = tempdir().unwrap();
    let db_path = tmp.path().to_path_buf();

    {
        let db = Vectrust::open(&db_path)?;
        let col = db.create_collection(
            "cli_test_col",
            CollectionConfig::builder(4, Metric::Cosine).build(),
        )?;
        col.insert(Point::new(1u64, vec![0.1, 0.2, 0.3, 0.4]))?;
        col.compact()?;
    }

    // Path to the compiled binary in debug profile
    let bin_path = if cfg!(windows) {
        "target/debug/vectrust.exe"
    } else {
        "target/debug/vectrust"
    };

    // 2. Test `vectrust --version`
    let version_output = Command::new(bin_path)
        .arg("--version")
        .output()
        .expect("Failed to execute vectrust --version");
    assert!(version_output.status.success());
    let version_str = String::from_utf8_lossy(&version_output.stdout);
    assert!(version_str.contains("vectrust"));

    // 3. Test `vectrust info <DB_PATH>`
    let info_output = Command::new(bin_path)
        .arg("info")
        .arg(&db_path)
        .output()
        .expect("Failed to execute vectrust info");
    assert!(info_output.status.success());
    let info_str = String::from_utf8_lossy(&info_output.stdout);
    assert!(info_str.contains("cli_test_col"));
    assert!(info_str.contains("Points:         1"));

    // 4. Test `vectrust check <DB_PATH>` (CRC32 validation)
    let check_output = Command::new(bin_path)
        .arg("check")
        .arg(&db_path)
        .output()
        .expect("Failed to execute vectrust check");
    assert!(check_output.status.success());
    let check_str = String::from_utf8_lossy(&check_output.stdout);
    assert!(check_str.contains("PASSED - CRC32 OK"));

    // 5. Test `vectrust benchmark`
    let bench_output = Command::new(bin_path)
        .args(["benchmark", "--dim", "8", "--points", "50", "--queries", "10", "--k", "3"])
        .output()
        .expect("Failed to execute vectrust benchmark");
    assert!(bench_output.status.success());
    let bench_str = String::from_utf8_lossy(&bench_output.stdout);
    assert!(bench_str.contains("Recall @ 3"));
    assert!(bench_str.contains("Flat Index"));
    assert!(bench_str.contains("HNSW Proximity"));

    Ok(())
}
