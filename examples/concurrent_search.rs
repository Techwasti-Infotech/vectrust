//! Concurrent Search and Ingestion Benchmark Example
//!
//! Demonstrates Vectrust's multi-threaded concurrency model:
//! Multiple threads performing searches in parallel with Rayon batch execution
//! while background workers concurrently stream new vector points into the collection.
//!
//! Run with: `cargo run --release --example concurrent_search`

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};
use rand::Rng;
use vectrust::prelude::*;

fn main() -> vectrust::Result<()> {
    println!("=== Vectrust Concurrency & High Throughput Demo ===");

    let dim = 32;
    let initial_points = 2_000;
    let db = Vectrust::in_memory();
    let config = CollectionConfig::builder(dim, Metric::Cosine)
        .index_type(IndexType::Hnsw)
        .hnsw_config(HnswConfig {
            m: 16,
            m0: 32,
            ef_construction: 64,
            ef_search: 32,
        })
        .build();

    let collection = Arc::new(db.create_collection("concurrent_hnsw", config)?);
    println!("Indexing {} baseline vectors (dim={})...", initial_points, dim);

    let mut baseline_points = Vec::with_capacity(initial_points);
    let mut rng = rand::thread_rng();
    for i in 0..initial_points {
        let v: Vec<f32> = (0..dim).map(|_| rng.gen_range(-1.0..1.0)).collect();
        baseline_points.push(Point::new(i as u64, v));
    }
    collection.insert_batch(baseline_points)?;
    println!("Initial vectors indexed. Starting concurrency benchmark...\n");

    let running = Arc::new(AtomicBool::new(true));
    let total_searches = Arc::new(AtomicUsize::new(0));
    let total_inserts = Arc::new(AtomicUsize::new(0));
    let num_search_threads = 4;
    let num_writer_threads = 2;
    let mut handles = Vec::new();

    let start_time = Instant::now();

    // Spawn Concurrent Ingestion Workers
    for w_id in 0..num_writer_threads {
        let col = Arc::clone(&collection);
        let run = Arc::clone(&running);
        let ins_counter = Arc::clone(&total_inserts);

        handles.push(thread::spawn(move || {
            let mut r = rand::thread_rng();
            let mut id = 10_000 + w_id * 10_000;
            while run.load(Ordering::Relaxed) {
                let v: Vec<f32> = (0..dim).map(|_| r.gen_range(-1.0..1.0)).collect();
                if col.insert(Point::new(id as u64, v)).is_ok() {
                    ins_counter.fetch_add(1, Ordering::Relaxed);
                }
                id += 1;
                thread::sleep(Duration::from_micros(500));
            }
        }));
    }

    // Spawn Concurrent Reader Workers
    for _ in 0..num_search_threads {
        let col = Arc::clone(&collection);
        let run = Arc::clone(&running);
        let search_counter = Arc::clone(&total_searches);

        handles.push(thread::spawn(move || {
            let mut r = rand::thread_rng();
            while run.load(Ordering::Relaxed) {
                let q: Vec<f32> = (0..dim).map(|_| r.gen_range(-1.0..1.0)).collect();
                let query = SearchQuery::new(q, 10);
                if let Ok(res) = col.search(query) {
                    assert_eq!(res.len(), 10);
                    search_counter.fetch_add(1, Ordering::Relaxed);
                }
            }
        }));
    }

    // Run benchmark for 2 seconds
    thread::sleep(Duration::from_secs(2));
    running.store(false, Ordering::Relaxed);

    for h in handles {
        h.join().unwrap();
    }

    let elapsed = start_time.elapsed().as_secs_f64();
    let searches = total_searches.load(Ordering::Relaxed);
    let inserts = total_inserts.load(Ordering::Relaxed);

    println!("Concurrency Test Results (over {:.2}s):", elapsed);
    println!("  Total Completed Searches: {:>8}", searches);
    println!("  Queries Per Second (QPS): {:>8.0} searches/sec", searches as f64 / elapsed);
    println!("  Concurrent Points Ingested: {:>6}", inserts);
    println!("  Final Collection Size:    {:>8} points", collection.count());

    // Batch Search Demo with Rayon
    println!("\nDemonstrating Rayon parallel batch search (50 queries in parallel)...");
    let batch_queries: Vec<SearchQuery> = (0..50)
        .map(|_| {
            let q: Vec<f32> = (0..dim).map(|_| rng.gen_range(-1.0..1.0)).collect();
            SearchQuery::new(q, 5)
        })
        .collect();

    let batch_start = Instant::now();
    let batch_results = collection.search_batch(batch_queries)?;
    let batch_elapsed = batch_start.elapsed();

    println!(
        "Processed 50 batch queries in {:?}, avg latency per query: {:?}",
        batch_elapsed,
        batch_elapsed / 50
    );
    assert_eq!(batch_results.len(), 50);

    Ok(())
}
