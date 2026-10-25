# Vectrust Documentation

Welcome to the official documentation for **Vectrust**, a high-performance, embedded vector database engine written in pure Rust.

```
       __     __        _                       _   
       \ \   / /__  ___| |_ _ __ _   _ ___  ___| |_ 
        \ \ / / _ \/ __| __| '__| | | / __|/ __| __|
         \ V /  __/ (__| |_| |  | |_| \__ \ (__| |_ 
          \_/ \___|\___|\__|_|   \__,_|___/\___|\__|
      High-Performance Embedded Vector Database Engine
```

---

## Documentation Sections

- [**Getting Started**](getting_started.md): Installation, first database creation, ingestion, search, and deletion.
- [**Local Development Setup**](local_setup.md): Complete guide for setting up Rust, building executables, and running tests.
- [**Contributing Guidelines**](../CONTRIBUTING.md): Workflow, coding standards, issue reporting, and pull request checklist.
- [**Architecture & Internals**](architecture.md): Deep-dive into SIMD distance metrics, HNSW graph mechanics, WAL format, and concurrency design.
- [**Metadata Filtering**](metadata_filtering.md): Complete guide to structured predicates, nested JSON dot-paths, and boolean filters.
- [**Benchmarks & Tuning**](benchmarks.md): Throughput benchmarks, QPS, latency vs recall analysis, and parameter tuning guide.
- [**API Reference**](api_reference.md): Detailed reference documentation for all public structs, traits, and functions.

---

## Key Highlights

- **Embedded First**: Runs directly in-process within your application. No servers, no Docker, zero network overhead.
- **State-of-the-art ANN**: Hierarchical Navigable Small World (HNSW) graph index with sub-millisecond query latency.
- **Exact Ground Truth**: Flat brute-force linear scan index powered by multi-threaded Rayon execution.
- **Crash-Resilient Persistence**: Append-only Write-Ahead Log (WAL) and atomic snapshots with CRC32 integrity checks.
- **Rich Metadata Filtering**: Pre-filtering and filter-aware graph traversal over nested JSON payloads.
- **Concurrent & Thread-Safe**: Concurrent reader architecture using `parking_lot::RwLock` and parallel Rayon batch search.
- **Pure Rust**: Zero C/C++ dependencies. Works out-of-the-box across Linux, macOS, and Windows.
