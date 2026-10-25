# Vectrust Architecture & Engine Internals

This document details the architectural principles, internal data structures, storage formats, and concurrency guarantees that power **Vectrust**.

---

## 1. System Overview

Vectrust is designed as an embedded, zero-dependency, library-first vector database engine. It compiles directly into your Rust application without requiring external server processes, network daemons, or C++ runtimes.

```
+---------------------------------------------------------------------------------+
|                                User Application                                 |
+---------------------------------------------------------------------------------+
                                      |
                                      v
+---------------------------------------------------------------------------------+
|                                 Vectrust DB                                     |
|  - Collection Lifecycle Management                                              |
|  - Multi-Collection Namespace Routing                                            |
+---------------------------------------------------------------------------------+
                                      |
                 +--------------------+--------------------+
                 |                                         |
                 v                                         v
+----------------------------------+     +----------------------------------+
|           Collection A           |     |           Collection B           |
|  - Arc<RwLock<CollectionInner>>  |     |  - Arc<RwLock<CollectionInner>>  |
+----------------------------------+     +----------------------------------+
                 |
     +-----------+-----------+----------------------+
     |                       |                      |
     v                       v                      v
+------------------+   +-------------------+  +-----------------------+
|  Index Engine    |   |  Metadata Store   |  |   Storage Engine      |
|  - HNSW (ANN)    |   |  - JSON Payloads  |  |  - Write-Ahead Log    |
|  - Flat (Exact)  |   |  - Path Resolver  |  |  - Atomic Snapshots   |
|  - Auto-Vec SIMD |   |  - Predicate Eval |  |  - CRC32 Checksums    |
+------------------+   +-------------------+  +-----------------------+
```

---

## 2. Distance Computation & SIMD Vectorization

Vectrust supports four core distance metrics in `vectrust::metric::Metric`:
- **Euclidean (L2)**: $d(a, b) = \sqrt{\sum (a_i - b_i)^2}$
- **Cosine**: $d(a, b) = 1.0 - \frac{a \cdot b}{\|a\| \|b\|}$
- **Dot Product / MIPS**: $d(a, b) = - (a \cdot b)$
- **Manhattan (L1)**: $d(a, b) = \sum |a_i - b_i|$

### Loop Unrolling & Instruction-Level Parallelism (ILP)
Modern CPUs can execute multiple vector instructions per cycle (e.g. AVX2, AVX-512, ARM NEON) if data dependencies between successive loop iterations are decoupled.

Vectrust implements distance loops with an 8-way parallel accumulator pattern:
```rust
let chunks_a = a.chunks_exact(8);
let chunks_b = b.chunks_exact(8);
let mut sum0 = 0.0f32;
let mut sum1 = 0.0f32;
// ...
let mut sum7 = 0.0f32;

for (ca, cb) in chunks_a.zip(chunks_b) {
    sum0 += (ca[0] - cb[0]) * (ca[0] - cb[0]);
    sum1 += (ca[1] - cb[1]) * (ca[1] - cb[1]);
    // ...
    sum7 += (ca[7] - cb[7]) * (ca[7] - cb[7]);
}
let total = (sum0 + sum1) + (sum2 + sum3) + (sum4 + sum5) + (sum6 + sum7);
```
This loop structure allows LLVM to vectorize across FMA (Fused Multiply-Add) registers with zero loop-carried dependency stalls.

---

## 3. Indexing Subsystem

Vectrust provides two complementary vector indexing engines implementing `VectorIndex`:

### 3.1 Flat Index (`FlatIndex`)
- **Type**: Brute-force exhaustive search.
- **Recall**: Guaranteed 100% ground-truth accuracy.
- **Storage**: Dense contiguous buffer of 32-bit floats.
- **Parallelism**: Automatically delegates to Rayon (`par_iter`) when indexing larger collections ($N > 2000$).
- **Use cases**: Baseline ground-truth evaluation, small collections ($< 10,000$ points), or scenarios requiring mathematically exact recall.

### 3.2 Hierarchical Navigable Small World (`HnswIndex`)
- **Type**: Multi-layer navigable proximity graph based on Malkov & Yashunin (2018).
- **Complexity**: $O(\log N)$ search time.
- **Layer Distribution**: Exponential decay using $m_L = \frac{1}{\ln(M)}$. Nodes are assigned random top layers $l \sim \text{Geometric}(m_L)$.
- **Graph Traversal**:
  1. **Coarse Routing**: Starts at `entry_point` in `max_level`. Greedily steps to the single nearest neighbor at each layer until descending to Layer 1.
  2. **Fine-grained Layer 0 Search**: Explores base layer 0 using a candidate priority queue bounded by `ef_search`.
  3. **Edge Pruning**: Maintains at most $M$ connections for higher layers and $M_0 = 2M$ connections at Layer 0.

### 3.3 Pre-Filtering Graph Traversal
In many vector databases, applying a filter causes one of two issues:
1. **Post-filtering**: Run ANN search, then discard non-matching candidates. If the filter is restrictive (e.g. 1% match rate), this returns fewer than $k$ results or empty sets.
2. **Disconnected Traversal**: If non-matching nodes are blocked from graph routing, connectivity breaks and search gets trapped in local minima.

**Vectrust's Approach**:
During graph exploration, non-matching nodes are utilized for routing (preventing disconnected partitions), while only candidates satisfying `filter.matches(payload)` enter the dynamic candidate priority queue. If the filter is hyper-selective, an automatic secondary fallback guarantees that up to `top_k` results are retrieved without degraded recall.

---

## 4. Storage Engine & Crash Resilience

Vectrust combines an append-only **Write-Ahead Log (WAL)** with **Atomic Snapshots** and **CRC32** validation.

```
Directory Layout:
data_dir/
└── collections/
    └── my_collection/
        ├── snapshot.bin      # Compressed binary state dump (CRC32 validated)
        ├── snapshot.tmp      # Staging file during atomic snapshot creation
        └── wal.log           # Append-only journal of operations post-snapshot
```

### 4.1 Write-Ahead Log (WAL)
Every mutation (`insert`, `delete`, `update_payload`) is appended to `wal.log` before being applied in memory:
```
+----------------+----------------+----------------+--------------------+
|  Record Length |  CRC32 Hash    |  Record Type   |  Bincode Payload   |
|   (4 bytes)    |   (4 bytes)    |   (1 byte)     |     (N bytes)      |
+----------------+----------------+----------------+--------------------+
```
- **Durability**: Configurable `sync_on_write` flag controls whether `fsync()` / `FlushFileBuffers()` is executed on every mutation.
- **Recovery**: On database restart, the WAL is replayed sequentially. If a system crash occurred during a write, the CRC32 check fails on the incomplete trailing frame, cleanly preserving all prior committed transactions.

### 4.2 Atomic Snapshots
When calling `collection.compact()`:
1. The full state (configuration, vectors, graph connectivity, metadata) is serialized.
2. Written to a temporary file (`snapshot.tmp`) with a 4-byte magic header (`VECS`), 2-byte version, 4-byte CRC32, and 8-byte payload length.
3. The temporary file is flushed and synced to disk.
4. Atomically replaces `snapshot.bin` (using atomic rename semantics on Windows and POSIX).
5. The `wal.log` file is safely truncated to 0 bytes.

---

## 5. Concurrency Model

Vectrust is optimized for high-concurrency read-heavy search workloads:

1. **Non-Poisoning Locks**: Utilizes `parking_lot::RwLock`, which avoids thread poisoning and provides microsecond-level lock acquisition overhead.
2. **Read Parallelism**: Multiple threads can execute `collection.search(...)` simultaneously without waiting on each other.
3. **Batch Vector Search**: `collection.search_batch(queries)` acquires a single shared read lock and evaluates all queries across available CPU cores using Rayon worker threads.
4. **Thread-Safe Handles**: `Collection` and `Vectrust` wrap internal state in `Arc`, making cloning cheap and thread-safe.
