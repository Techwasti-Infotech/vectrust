//! HNSW vs Flat Index Benchmark Example
//!
//! Evaluates indexing time, query latency (µs/query), throughput,
//! and recall accuracy between Approximate Nearest Neighbor (HNSW)
//! and Exact Ground Truth (Flat brute-force linear scan).
//!
//! Run with: `cargo run --release --example hnsw_vs_flat_benchmark`

use std::collections::HashSet;
use std::time::Instant;
use rand::Rng;
use vectrust::prelude::*;

fn main() -> vectrust::Result<()> {
    println!("============================================================");
    println!("     Vectrust Performance: HNSW vs Flat Index Benchmark     ");
    println!("============================================================\n");

    let dim = 64;
    let num_points = 5_000;
    let num_test_queries = 100;
    let top_k = 10;

    println!("Dataset Parameters:");
    println!("  - Dimension:           {}", dim);
    println!("  - Indexed Vectors:     {}", num_points);
    println!("  - Evaluation Queries:  {}", num_test_queries);
    println!("  - Top-K:               {}", top_k);
    println!("  - Metric:              Cosine Similarity\n");

    let db = Vectrust::in_memory();

    // 1. Generate random synthetic vector dataset
    println!("Generating synthetic vector dataset...");
    let mut rng = rand::thread_rng();
    let mut points = Vec::with_capacity(num_points);
    for i in 0..num_points {
        let v: Vec<f32> = (0..dim).map(|_| rng.gen_range(-1.0..1.0)).collect();
        points.push(Point::new(i as u64, v));
    }

    let mut test_queries = Vec::with_capacity(num_test_queries);
    for _ in 0..num_test_queries {
        let q: Vec<f32> = (0..dim).map(|_| rng.gen_range(-1.0..1.0)).collect();
        test_queries.push(SearchQuery::new(q, top_k));
    }

    // 2. Benchmark Flat Index (Ground Truth)
    println!("\n[1/2] Benchmarking Flat Index (Ground Truth)...");
    let flat_config = CollectionConfig::builder(dim, Metric::Cosine)
        .index_type(IndexType::Flat)
        .build();
    let flat_col = db.create_collection("bench_flat", flat_config)?;

    let flat_build_start = Instant::now();
    flat_col.insert_batch(points.clone())?;
    let flat_build_time = flat_build_start.elapsed();
    println!("  -> Flat build/insert time: {:?}", flat_build_time);

    let flat_search_start = Instant::now();
    let mut ground_truth_results = Vec::with_capacity(num_test_queries);
    for q in &test_queries {
        let res = flat_col.search(q.clone())?;
        ground_truth_results.push(res);
    }
    let flat_search_time = flat_search_start.elapsed();
    let flat_avg_latency = flat_search_time / num_test_queries as u32;
    println!("  -> Flat search time ({} queries): {:?}", num_test_queries, flat_search_time);
    println!("  -> Flat avg latency per query:    {:?}", flat_avg_latency);

    // 3. Benchmark HNSW Index (Approximate Nearest Neighbor)
    println!("\n[2/2] Benchmarking HNSW Index (ANN)...");
    let hnsw_config = CollectionConfig::builder(dim, Metric::Cosine)
        .index_type(IndexType::Hnsw)
        .hnsw_config(HnswConfig {
            m: 16,
            m0: 32,
            ef_construction: 100,
            ef_search: 64,
        })
        .build();
    let hnsw_col = db.create_collection("bench_hnsw", hnsw_config)?;

    let hnsw_build_start = Instant::now();
    hnsw_col.insert_batch(points)?;
    let hnsw_build_time = hnsw_build_start.elapsed();
    println!("  -> HNSW build time: {:?}", hnsw_build_time);

    let hnsw_search_start = Instant::now();
    let mut hnsw_results = Vec::with_capacity(num_test_queries);
    for q in &test_queries {
        let res = hnsw_col.search(q.clone())?;
        hnsw_results.push(res);
    }
    let hnsw_search_time = hnsw_search_start.elapsed();
    let hnsw_avg_latency = hnsw_search_time / num_test_queries as u32;
    println!("  -> HNSW search time ({} queries): {:?}", num_test_queries, hnsw_search_time);
    println!("  -> HNSW avg latency per query:    {:?}", hnsw_avg_latency);

    // 4. Calculate Recall@K Accuracy
    let mut total_recall = 0.0;
    for (hnsw_res, flat_res) in hnsw_results.iter().zip(ground_truth_results.iter()) {
        let truth_ids: HashSet<_> = flat_res.iter().map(|r| &r.id).collect();
        let matches = hnsw_res.iter().filter(|r| truth_ids.contains(&r.id)).count();
        total_recall += matches as f64 / top_k as f64;
    }
    let avg_recall = (total_recall / num_test_queries as f64) * 100.0;

    let speedup = flat_search_time.as_secs_f64() / hnsw_search_time.as_secs_f64();

    println!("\n=================== Summary ===================");
    println!("  * HNSW Recall@{}:      {:.2}%", top_k, avg_recall);
    println!("  * Flat Search Latency:  {:?}", flat_avg_latency);
    println!("  * HNSW Search Latency:  {:?}", hnsw_avg_latency);
    println!("  * Search Speedup:       {:.2}x faster", speedup);
    println!("===============================================");

    Ok(())
}
