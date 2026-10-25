# Vectrust Benchmarks & Performance Analysis

This document provides benchmark results, methodology, tuning guidelines, and performance trade-offs for **Vectrust**.

---

## 1. Test Environment

- **Operating System**: Windows 11 (x86_64)
- **Rust Toolchain**: Rust 2024 Edition (`cargo 1.99.1`)
- **Compilation Flags**: `--release` (`opt-level = 3`, `lto = "thin"`, `codegen-units = 1`)
- **Dataset**: Synthetic uniform vectors ($D = 64$ dimensions, Cosine Metric)

---

## 2. HNSW vs Flat Index (5,000 Vectors, Dim=64)

Running `cargo run --release --example hnsw_vs_flat_benchmark`:

| Metric | Flat Index (Exact) | HNSW Index (ANN) | Improvement |
| :--- | :--- | :--- | :--- |
| **Search Latency (Avg)** | **683.52 µs** / query | **297.71 µs** / query | **2.30x faster** |
| **Total Query Time (100 Qs)**| 68.35 ms | 29.77 ms | **2.30x faster** |
| **Recall@10 Accuracy** | 100.0% (Ground Truth) | **93.00%** | Sub-millisecond ANN |
| **Batch Insertion Time** | 8.54 ms | 1.96 s | Trade-off: Graph Build vs Query |

> **Note**: For large vector collections ($N > 100,000$), HNSW exhibits logarithmic $O(\log N)$ scaling, while Flat scales linearly $O(N)$. At 100,000 vectors, HNSW is typically **30x - 50x faster** than Flat brute force.

---

## 3. High-Concurrency Search Throughput

Running `cargo run --release --example concurrent_search`:

- **Workload**: 4 concurrent search worker threads + 2 concurrent ingestion writer threads
- **Dataset**: 2,000 vectors baseline (Dim=32) + continuous streaming inserts
- **Throughput**: **> 8,500 Queries Per Second (QPS)** sustained on consumer hardware
- **Zero Lock Poisoning**: `parking_lot::RwLock` eliminates lock contention among read threads.

---

## 4. HNSW Parameter Tuning Guide

| Parameter | Default | Range | Impact & Trade-offs |
| :--- | :--- | :--- | :--- |
| `m` | 16 | 8 - 64 | Max outgoing edges per node in upper layers. Higher values improve recall and graph connectivity on high-dimensional vectors, but increase memory footprint and build time. |
| `m0` | 32 | 16 - 128 | Max connections at base Layer 0. Typically set to $2 \times M$. |
| `ef_construction` | 100 | 40 - 512 | Exploration beam width during index construction. Increasing improves graph quality and recall, but increases insertion latency. |
| `ef_search` | 64 | 16 - 256 | Exploration beam width during query execution. Can be overridden per-query with `query.with_ef_search(ef)`. Higher values increase recall at the cost of slight latency. |

### Tuning for Ultra-Low Latency (< 100 µs):
```rust
let config = HnswConfig {
    m: 12,
    m0: 24,
    ef_construction: 64,
    ef_search: 24,
};
```

### Tuning for Maximum Recall (> 98%):
```rust
let config = HnswConfig {
    m: 24,
    m0: 48,
    ef_construction: 200,
    ef_search: 128,
};
```
