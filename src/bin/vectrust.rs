use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::Instant;

use rand::Rng;
use serde_json::Value;

use vectrust::prelude::*;
use vectrust::storage::{load_snapshot, Wal};

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn print_banner() {
    println!(
        "\x1b[38;2;99;102;241m"
    );
    println!(r#"  _    __          __                 __ "#);
    println!(r#" | |  / /__  _____/ /________  _______/ /_"#);
    println!(r#" | | / / _ \/ ___/ __/ ___/ / / / ___/ __/"#);
    println!(r#" | |/ /  __/ /__/ /_/ /  / /_/ (__  ) /_  "#);
    println!(r#" |___/\___/\___/\__/_/   \__,_/____/\__/  "#);
    println!("\x1b[0m");
    println!(
        " High-Performance Embedded Vector Database Engine (v{})",
        VERSION
    );
    println!(" Repository: https://github.com/techwastiinfotech/vectrust\n");
}

fn print_help() {
    print_banner();
    println!("USAGE:");
    println!("    vectrust <SUBCOMMAND> [OPTIONS]\n");
    println!("SUBCOMMANDS:");
    println!("    info <DIR>           Inspect an existing Vectrust database directory");
    println!("    check <DIR>          Verify CRC32 integrity of WAL logs and snapshots");
    println!("    benchmark [OPTIONS]  Run embedded HNSW vs Flat performance benchmark");
    println!("    repl [DIR]           Start interactive database terminal (memory or path)");
    println!("    version              Print version information");
    println!("    help                 Print this help manual\n");
    println!("BENCHMARK OPTIONS:");
    println!("    --dim <N>            Vector dimension (default: 64)");
    println!("    --points <N>         Number of vectors to index (default: 2000)");
    println!("    --queries <N>        Number of query searches (default: 100)");
    println!("    --k <N>              Top-K nearest neighbors (default: 10)\n");
    println!("EXAMPLES:");
    println!("    vectrust benchmark --dim 128 --points 5000 --k 10");
    println!("    vectrust info ./data");
    println!("    vectrust check ./data");
    println!("    vectrust repl ./data");
}

fn cmd_info(db_path: &Path) {
    if !db_path.exists() {
        eprintln!("\x1b[31mError:\x1b[0m Database path does not exist: {}", db_path.display());
        std::process::exit(1);
    }

    println!("\x1b[1m\x1b[36m=== Inspecting Vectrust Database: {} ===\x1b[0m", db_path.display());

    let db = match Vectrust::open(db_path) {
        Ok(db) => db,
        Err(e) => {
            eprintln!("\x1b[31mFailed to open database:\x1b[0m {}", e);
            std::process::exit(1);
        }
    };

    let collections = db.list_collections();
    if collections.is_empty() {
        println!("No active collections found in this database.");
        return;
    }

    println!("Found {} collection(s):\n", collections.len());
    for name in collections {
        if let Some(col) = db.get_collection(&name) {
            let cfg = col.config();
            let count = col.count();
            let col_dir = db_path.join("collections").join(&name);
            let snap_file = col_dir.join("snapshot.bin");
            let wal_file = col_dir.join("wal.log");

            let snap_size = fs::metadata(&snap_file).map(|m| m.len()).unwrap_or(0);
            let wal_size = fs::metadata(&wal_file).map(|m| m.len()).unwrap_or(0);

            println!("  • Collection: \x1b[32m{}\x1b[0m", name);
            println!("      Dimension:      {}", cfg.dimension);
            println!("      Metric:         {:?}", cfg.metric);
            println!("      Index Engine:   {:?}", cfg.index_type);
            println!("      Points:         {}", count);
            println!("      Snapshot File:  {} bytes", snap_size);
            println!("      WAL Journal:    {} bytes", wal_size);
            if cfg.index_type == IndexType::Hnsw {
                let hnsw_cfg = cfg.hnsw_config;
                println!(
                    "      HNSW Config:    M={}, M0={}, ef_construction={}, ef_search={}",
                    hnsw_cfg.m, hnsw_cfg.m0, hnsw_cfg.ef_construction, hnsw_cfg.ef_search
                );
            }
            println!();
        }
    }
}

fn cmd_check(db_path: &Path) {
    if !db_path.exists() {
        eprintln!("\x1b[31mError:\x1b[0m Database path does not exist: {}", db_path.display());
        std::process::exit(1);
    }

    println!("\x1b[1m\x1b[36m=== Validating Data Integrity (CRC32 Verification): {} ===\x1b[0m", db_path.display());
    let collections_dir = db_path.join("collections");
    if !collections_dir.exists() {
        println!("No collections directory found.");
        return;
    }

    let entries = match fs::read_dir(&collections_dir) {
        Ok(e) => e,
        Err(e) => {
            eprintln!("Failed to read collections directory: {}", e);
            std::process::exit(1);
        }
    };

    let mut total_collections = 0;
    let mut passed = 0;

    for entry in entries.flatten() {
        if entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false) {
            total_collections += 1;
            let name = entry.file_name().to_string_lossy().to_string();
            let col_dir = entry.path();
            let snap_file = col_dir.join("snapshot.bin");
            let wal_file = col_dir.join("wal.log");

            print!("  • Checking collection '{}' ... ", name);
            let mut ok = true;

            if snap_file.exists() {
                match load_snapshot(&snap_file) {
                    Ok(Some(_)) => {}
                    Ok(None) => {}
                    Err(e) => {
                        println!("\x1b[31mSnapshot CRC corrupted: {}\x1b[0m", e);
                        ok = false;
                    }
                }
            }

            if wal_file.exists() {
                match Wal::read_all(&wal_file) {
                    Ok(_) => {}
                    Err(e) => {
                        println!("\x1b[31mWAL journal corrupted: {}\x1b[0m", e);
                        ok = false;
                    }
                }
            }

            if ok {
                println!("\x1b[32m[PASSED - CRC32 OK]\x1b[0m");
                passed += 1;
            }
        }
    }

    println!(
        "\nIntegrity check complete: {}/{} collections passed.",
        passed, total_collections
    );
}

fn cmd_benchmark(dim: usize, num_points: usize, num_queries: usize, k: usize) {
    print_banner();
    println!(
        "\x1b[1mBenchmarking Vectrust Embedded Engine (dim={}, points={}, queries={}, k={})\x1b[0m\n",
        dim, num_points, num_queries, k
    );

    let mut rng = rand::thread_rng();

    // 1. Generate synthetic dataset
    print!("Generating {} random unit vectors ... ", num_points);
    io::stdout().flush().unwrap();
    let gen_start = Instant::now();

    let points: Vec<Point> = (0..num_points)
        .map(|i| {
            let mut v: Vec<f32> = (0..dim).map(|_| rng.gen_range(-1.0..1.0)).collect();
            let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
            if norm > 0.0 {
                for x in &mut v {
                    *x /= norm;
                }
            }
            Point::new(i as u64, v)
        })
        .collect();
    println!("done in {:.2}ms", gen_start.elapsed().as_secs_f64() * 1000.0);

    // 2. Generate queries
    let queries: Vec<Vec<f32>> = (0..num_queries)
        .map(|_| {
            let mut v: Vec<f32> = (0..dim).map(|_| rng.gen_range(-1.0..1.0)).collect();
            let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
            if norm > 0.0 {
                for x in &mut v {
                    *x /= norm;
                }
            }
            v
        })
        .collect();

    // 3. Flat (Exact Linear Scan) Baseline
    println!("\n\x1b[33m[1/2] Flat Index (Ground Truth Linear Scan)...\x1b[0m");
    let flat_cfg = CollectionConfig::builder(dim, Metric::Cosine)
        .index_type(IndexType::Flat)
        .build();
    let flat_col = Collection::in_memory(flat_cfg);

    let flat_build_start = Instant::now();
    for p in &points {
        flat_col.insert(p.clone()).unwrap();
    }
    let flat_build_time = flat_build_start.elapsed();
    println!("  Build time: {:.2}ms", flat_build_time.as_secs_f64() * 1000.0);

    let flat_search_start = Instant::now();
    let mut ground_truth: Vec<Vec<PointId>> = Vec::with_capacity(num_queries);
    for q in &queries {
        let results = flat_col.search(SearchQuery::new(q.clone(), k)).unwrap();
        ground_truth.push(results.into_iter().map(|r| r.id).collect());
    }
    let flat_search_time = flat_search_start.elapsed();
    let flat_qps = (num_queries as f64) / flat_search_time.as_secs_f64();
    let flat_avg_lat_us = (flat_search_time.as_secs_f64() * 1_000_000.0) / (num_queries as f64);
    println!("  Search throughput: {:.0} QPS (avg latency: {:.1}µs)", flat_qps, flat_avg_lat_us);

    // 4. HNSW (Hierarchical Navigable Small World Proximity Graph)
    println!("\n\x1b[34m[2/2] HNSW Index (Approximate Nearest Neighbor)...\x1b[0m");
    let hnsw_cfg = CollectionConfig::builder(dim, Metric::Cosine)
        .index_type(IndexType::Hnsw)
        .hnsw_config(HnswConfig {
            m: 16,
            m0: 32,
            ef_construction: 64,
            ef_search: 40,
        })
        .build();
    let hnsw_col = Collection::in_memory(hnsw_cfg);

    let hnsw_build_start = Instant::now();
    for p in &points {
        hnsw_col.insert(p.clone()).unwrap();
    }
    let hnsw_build_time = hnsw_build_start.elapsed();
    println!("  Build time: {:.2}ms", hnsw_build_time.as_secs_f64() * 1000.0);

    let hnsw_search_start = Instant::now();
    let mut total_recall = 0.0;
    for (i, q) in queries.iter().enumerate() {
        let results = hnsw_col.search(SearchQuery::new(q.clone(), k)).unwrap();
        let retrieved_ids: std::collections::HashSet<PointId> = results.into_iter().map(|r| r.id).collect();
        let gt = &ground_truth[i];
        let hits = gt.iter().filter(|id| retrieved_ids.contains(id)).count();
        if !gt.is_empty() {
            total_recall += (hits as f64) / (gt.len() as f64);
        }
    }
    let hnsw_search_time = hnsw_search_start.elapsed();
    let hnsw_qps = (num_queries as f64) / hnsw_search_time.as_secs_f64();
    let hnsw_avg_lat_us = (hnsw_search_time.as_secs_f64() * 1_000_000.0) / (num_queries as f64);
    let avg_recall = total_recall / (num_queries as f64);
    println!("  Search throughput: {:.0} QPS (avg latency: {:.1}µs)", hnsw_qps, hnsw_avg_lat_us);
    println!("  Recall @ {}:        \x1b[32m{:.2}%\x1b[0m", k, avg_recall * 100.0);

    // Summary Comparison Table
    println!("\n\x1b[1m+-------------------------+--------------------+--------------------+\x1b[0m");
    println!("\x1b[1m| Metric                  | Flat Index (Exact) | HNSW Proximity     |\x1b[0m");
    println!("\x1b[1m+-------------------------+--------------------+--------------------+\x1b[0m");
    println!(
        "| Build Time              | {:>15.2} ms | {:>15.2} ms |",
        flat_build_time.as_secs_f64() * 1000.0,
        hnsw_build_time.as_secs_f64() * 1000.0
    );
    println!(
        "| Search Latency          | {:>15.1} µs | {:>15.1} µs |",
        flat_avg_lat_us, hnsw_avg_lat_us
    );
    println!(
        "| Queries / Sec (QPS)     | {:>18.0} | {:>18.0} |",
        flat_qps, hnsw_qps
    );
    println!(
        "| Recall @ {:<2}             |            100.00% | {:>17.2}% |",
        k,
        avg_recall * 100.0
    );
    println!(
        "| Speedup Factor          |               1.0x | {:>16.1}x |",
        hnsw_qps / flat_qps.max(1.0)
    );
    println!("\x1b[1m+-------------------------+--------------------+--------------------+\x1b[0m\n");
}

fn cmd_repl(db_path: Option<PathBuf>) {
    print_banner();
    let db = match &db_path {
        Some(p) => {
            println!("Storage: \x1b[32mPersistent\x1b[0m at '{}'", p.display());
            match Vectrust::open(p) {
                Ok(db) => db,
                Err(e) => {
                    eprintln!("Failed to open database: {}", e);
                    std::process::exit(1);
                }
            }
        }
        None => {
            println!("Storage: \x1b[33mEphemeral In-Memory\x1b[0m (changes discarded on exit)");
            Vectrust::in_memory()
        }
    };

    println!("Type 'help' for available commands, or 'exit' to quit.\n");

    let stdin = io::stdin();
    let mut line = String::new();

    loop {
        print!("\x1b[36mvectrust>\x1b[0m ");
        io::stdout().flush().unwrap();
        line.clear();

        if stdin.read_line(&mut line).is_err() || line.trim().is_empty() {
            if line.is_empty() {
                break; // EOF
            }
            continue;
        }

        let parts: Vec<&str> = line.trim().split_whitespace().collect();
        if parts.is_empty() {
            continue;
        }

        match parts[0] {
            "exit" | "quit" | "q" => {
                println!("Flushing collections and exiting...");
                let _ = db.flush_all();
                break;
            }
            "help" => {
                println!("REPL Commands:");
                println!("  list                                   List collections");
                println!("  create <name> <dim> [metric] [index]   Create a collection (e.g. create docs 4 cosine hnsw)");
                println!("  insert <name> <id> <v1,v2,...> [json]  Insert vector and optional JSON payload");
                println!("  search <name> <k> <v1,v2,...> [filter] Search top-K nearest neighbors with optional filter");
                println!("  get <name> <id>                        Fetch a point by ID");
                println!("  delete <name> <id>                     Delete a point by ID");
                println!("  count <name>                           Show number of points in collection");
                println!("  drop <name>                            Delete a collection");
                println!("  flush                                  Flush/snapshot all collections to disk");
                println!("  exit / quit                            Exit REPL");
            }
            "list" => {
                let cols = db.list_collections();
                println!("Collections ({}):", cols.len());
                for c in cols {
                    let count = db.get_collection(&c).map(|col| col.count()).unwrap_or(0);
                    println!("  • {} (points: {})", c, count);
                }
            }
            "create" => {
                if parts.len() < 3 {
                    println!("Usage: create <name> <dim> [cosine|euclidean|dot|manhattan] [hnsw|flat]");
                    continue;
                }
                let name = parts[1];
                let dim: usize = match parts[2].parse() {
                    Ok(d) => d,
                    Err(_) => {
                        println!("Invalid dimension: {}", parts[2]);
                        continue;
                    }
                };
                let metric = match parts.get(3).copied().unwrap_or("cosine") {
                    "euclidean" | "l2" => Metric::Euclidean,
                    "dot" | "mips" => Metric::DotProduct,
                    "manhattan" | "l1" => Metric::Manhattan,
                    _ => Metric::Cosine,
                };
                let idx_type = match parts.get(4).copied().unwrap_or("hnsw") {
                    "flat" => IndexType::Flat,
                    _ => IndexType::Hnsw,
                };

                let config = CollectionConfig::builder(dim, metric)
                    .index_type(idx_type)
                    .build();

                match db.create_collection(name, config) {
                    Ok(_) => println!("\x1b[32mCreated collection '{}' (dim={}, {:?}, {:?})\x1b[0m", name, dim, metric, idx_type),
                    Err(e) => println!("\x1b[31mError:\x1b[0m {}", e),
                }
            }
            "insert" => {
                if parts.len() < 4 {
                    println!("Usage: insert <name> <id> <v1,v2,...> [payload_json]");
                    continue;
                }
                let col_name = parts[1];
                let id_str = parts[2];
                let vec_str = parts[3];

                let id = if let Ok(n) = id_str.parse::<u64>() {
                    PointId::U64(n)
                } else {
                    PointId::String(id_str.to_string())
                };

                let vector: std::result::Result<Vec<f32>, _> = vec_str.split(',').map(|s| s.trim().parse::<f32>()).collect();
                let vector = match vector {
                    Ok(v) => v,
                    Err(e) => {
                        println!("Invalid vector float: {}", e);
                        continue;
                    }
                };

                let payload: Option<Value> = if parts.len() > 4 {
                    let rest = parts[4..].join(" ");
                    serde_json::from_str(&rest).ok()
                } else {
                    None
                };

                if let Some(col) = db.get_collection(col_name) {
                    let mut pt = Point::new(id, vector);
                    if let Some(p) = payload {
                        pt = pt.with_payload(p);
                    }
                    match col.insert(pt) {
                        Ok(_) => println!("\x1b[32mInserted successfully.\x1b[0m"),
                        Err(e) => println!("\x1b[31mInsert failed:\x1b[0m {}", e),
                    }
                } else {
                    println!("Collection '{}' not found.", col_name);
                }
            }
            "search" => {
                if parts.len() < 4 {
                    println!("Usage: search <name> <k> <v1,v2,...> [filter_json]");
                    continue;
                }
                let col_name = parts[1];
                let k: usize = match parts[2].parse() {
                    Ok(v) => v,
                    Err(_) => {
                        println!("Invalid k: {}", parts[2]);
                        continue;
                    }
                };
                let vec_str = parts[3];
                let vector: std::result::Result<Vec<f32>, _> = vec_str.split(',').map(|s| s.trim().parse::<f32>()).collect();
                let vector = match vector {
                    Ok(v) => v,
                    Err(e) => {
                        println!("Invalid vector float: {}", e);
                        continue;
                    }
                };

                let filter: Option<Filter> = if parts.len() > 4 {
                    let rest = parts[4..].join(" ");
                    match serde_json::from_str::<Filter>(&rest) {
                        Ok(f) => Some(f),
                        Err(e) => {
                            println!("Invalid filter JSON: {}", e);
                            continue;
                        }
                    }
                } else {
                    None
                };

                if let Some(col) = db.get_collection(col_name) {
                    let mut q = SearchQuery::new(vector, k).include_payload(true);
                    if let Some(f) = filter {
                        q = q.with_filter(f);
                    }

                    let t0 = Instant::now();
                    match col.search(q) {
                        Ok(results) => {
                            let dt = t0.elapsed();
                            println!("Found {} results in {:.2}µs:", results.len(), dt.as_secs_f64() * 1_000_000.0);
                            for (i, r) in results.into_iter().enumerate() {
                                println!(
                                    "  [{}] ID: {:<12} Score / Distance: {:.6} | Payload: {}",
                                    i + 1,
                                    r.id,
                                    r.score,
                                    r.payload.map(|p| p.to_string()).unwrap_or_else(|| "null".to_string())
                                );
                            }
                        }
                        Err(e) => println!("\x1b[31mSearch failed:\x1b[0m {}", e),
                    }
                } else {
                    println!("Collection '{}' not found.", col_name);
                }
            }
            "get" => {
                if parts.len() < 3 {
                    println!("Usage: get <name> <id>");
                    continue;
                }
                let col_name = parts[1];
                let id_str = parts[2];
                let id = if let Ok(n) = id_str.parse::<u64>() {
                    PointId::U64(n)
                } else {
                    PointId::String(id_str.to_string())
                };

                if let Some(col) = db.get_collection(col_name) {
                    match col.get(id) {
                        Ok(Some(pt)) => {
                            println!("Point ID: {}", pt.id);
                            println!("Vector:   {:?}", pt.vector);
                            println!("Payload:  {}", pt.payload.map(|p| p.to_string()).unwrap_or_else(|| "none".to_string()));
                        }
                        Ok(None) => println!("Point not found."),
                        Err(e) => println!("Get error: {}", e),
                    }
                } else {
                    println!("Collection '{}' not found.", col_name);
                }
            }
            "delete" => {
                if parts.len() < 3 {
                    println!("Usage: delete <name> <id>");
                    continue;
                }
                let col_name = parts[1];
                let id_str = parts[2];
                let id = if let Ok(n) = id_str.parse::<u64>() {
                    PointId::U64(n)
                } else {
                    PointId::String(id_str.to_string())
                };

                if let Some(col) = db.get_collection(col_name) {
                    match col.delete(id) {
                        Ok(true) => println!("\x1b[32mDeleted point successfully.\x1b[0m"),
                        Ok(false) => println!("Point not found."),
                        Err(e) => println!("Delete error: {}", e),
                    }
                } else {
                    println!("Collection '{}' not found.", col_name);
                }
            }
            "count" => {
                if parts.len() < 2 {
                    println!("Usage: count <name>");
                    continue;
                }
                let col_name = parts[1];
                if let Some(col) = db.get_collection(col_name) {
                    println!("Collection '{}' points: {}", col_name, col.count());
                } else {
                    println!("Collection '{}' not found.", col_name);
                }
            }
            "drop" => {
                if parts.len() < 2 {
                    println!("Usage: drop <name>");
                    continue;
                }
                let col_name = parts[1];
                match db.drop_collection(col_name) {
                    Ok(true) => println!("\x1b[32mCollection dropped.\x1b[0m"),
                    Ok(false) => println!("Collection did not exist."),
                    Err(e) => println!("Drop failed: {}", e),
                }
            }
            "flush" => {
                match db.flush_all() {
                    Ok(_) => println!("\x1b[32mAll collections flushed and compacted.\x1b[0m"),
                    Err(e) => println!("Flush error: {}", e),
                }
            }
            other => {
                println!("Unknown command: '{}'. Type 'help' for command list.", other);
            }
        }
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        print_help();
        return;
    }

    match args[1].as_str() {
        "--version" | "-v" | "version" => {
            println!("vectrust {}", VERSION);
        }
        "--help" | "-h" | "help" => {
            print_help();
        }
        "info" => {
            if args.len() < 3 {
                eprintln!("Usage: vectrust info <DB_DIRECTORY>");
                std::process::exit(1);
            }
            cmd_info(Path::new(&args[2]));
        }
        "check" => {
            if args.len() < 3 {
                eprintln!("Usage: vectrust check <DB_DIRECTORY>");
                std::process::exit(1);
            }
            cmd_check(Path::new(&args[2]));
        }
        "benchmark" => {
            let mut dim = 64;
            let mut points = 2000;
            let mut queries = 100;
            let mut k = 10;

            let mut i = 2;
            while i < args.len() {
                match args[i].as_str() {
                    "--dim" if i + 1 < args.len() => {
                        dim = args[i + 1].parse().unwrap_or(dim);
                        i += 2;
                    }
                    "--points" if i + 1 < args.len() => {
                        points = args[i + 1].parse().unwrap_or(points);
                        i += 2;
                    }
                    "--queries" if i + 1 < args.len() => {
                        queries = args[i + 1].parse().unwrap_or(queries);
                        i += 2;
                    }
                    "--k" if i + 1 < args.len() => {
                        k = args[i + 1].parse().unwrap_or(k);
                        i += 2;
                    }
                    _ => i += 1,
                }
            }
            cmd_benchmark(dim, points, queries, k);
        }
        "repl" | "interactive" => {
            let path = if args.len() > 2 {
                Some(PathBuf::from(&args[2]))
            } else {
                None
            };
            cmd_repl(path);
        }
        other => {
            eprintln!("Unknown command: '{}'\n", other);
            print_help();
            std::process::exit(1);
        }
    }
}
