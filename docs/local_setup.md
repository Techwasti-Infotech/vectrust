# 🛠️ Vectrust Local Development Setup Guide

Welcome to the **Vectrust** development environment! This guide walks you through setting up your local machine to build, test, benchmark, and contribute to the Vectrust vector database engine.

---

## 1. System Prerequisites

Vectrust is written in 100% pure Rust and requires **Rust 1.75+ (Edition 2021)**. It has zero external C/C++ or system library dependencies.

### Install Rust & Cargo

If you don't already have Rust installed, use `rustup`:

```bash
# macOS / Linux
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Windows
# Download and run rustup-init.exe from https://rustup.rs
```

Verify your installation:

```bash
rustc --version   # Should report rustc 1.75.0 or higher
cargo --version   # Should report cargo 1.75.0 or higher
```

### Optional Recommended Tools

- **`cargo-watch`**: Automatically recompiles or runs tests on file change:
  ```bash
  cargo install cargo-watch
  ```
- **`cargo-criterion`**: Micro-benchmarking analysis tool:
  ```bash
  cargo install cargo-criterion
  ```

---

## 2. Clone the Repository

Clone the repository with Git:

```bash
git clone https://github.com/techwastiinfotech/vectrust.git
cd vectrust
```

---

## 3. Building the Project

### Build Debug Library & Binary

```bash
cargo build
```

This compiles both the library crate (`vectrust`) and the CLI binary (`vectrust`).

### Build Optimized Release Executable

For maximum SIMD loop unrolling and thin LTO optimization:

```bash
cargo build --release --bin vectrust
```

The resulting optimized executable will be located at:
- **Linux / macOS**: `./target/release/vectrust`
- **Windows**: `.\target\release\vectrust.exe`

Test running the executable:

```bash
cargo run --bin vectrust -- --help
```

---

## 4. Running the Test Suite

Vectrust has a comprehensive test suite covering SIMD vector math, HNSW graph construction, concurrent readers/writers, metadata filtering, and WAL/snapshot persistence.

### Run All Unit & Integration Tests

```bash
cargo test
```

### Run Tests with Full Stdout Output

```bash
cargo test -- --nocapture
```

### Run a Specific Integration Test

```bash
# Concurrency & thread safety test
cargo test --test concurrency_tests

# HNSW graph CRUD & recall test
cargo test --test hnsw_tests

# Metadata filtering logic test
cargo test --test metadata_filtering_tests

# WAL persistence & atomic snapshot compaction test
cargo test --test persistence_tests
```

### Continuous Test Watcher

Automatically re-run tests on every code edit:

```bash
cargo watch -x test
```

---

## 5. Running Examples & Benchmarks

Explore real-world usage patterns via the `examples/` directory:

```bash
# 1. Quickstart (inserting, searching, payload inspection)
cargo run --example quickstart

# 2. Metadata filtering with nested JSON dot-notation
cargo run --example metadata_filtering

# 3. High-concurrency benchmark with streaming writes and Rayon batch queries
cargo run --release --example concurrent_search

# 4. HNSW vs Flat ground truth accuracy and latency benchmark
cargo run --release --example hnsw_vs_flat_benchmark

# 5. Persistent storage with simulated crash recovery and WAL replay
cargo run --example persistent_storage
```

---

## 6. Code Quality & Formatting

Before opening a pull request, verify that your code adheres to project formatting and linting rules:

### Check Formatting

```bash
cargo fmt --check
```

To automatically format all code:

```bash
cargo fmt
```

### Run Clippy Linter

```bash
cargo clippy --all-targets --all-features -- -D warnings
```

---

## 7. Generating & Previewing Documentation

### Generate Rustdoc API Documentation

```bash
cargo doc --no-deps --all-features --open
```

### Preview the Interactive GitHub Pages Documentation

The `site/` directory contains the modern documentation website and live 2D vector playground:

```bash
# Using Python 3 built-in HTTP server:
python -m http.server 8080 --directory site

# Or using Node.js / npx serve:
npx serve site
```

Open your browser to [http://localhost:8080](http://localhost:8080) to interact with the documentation and simulator.

---

## 8. Directory Structure Overview

```text
vectrust/
├── Cargo.toml            # Package configuration and dependencies
├── README.md             # Project overview & badges
├── CONTRIBUTING.md       # Contribution guidelines & standards
├── .github/
│   ├── workflows/        # GitHub Actions (CI, release, pages)
│   └── ISSUE_TEMPLATE/   # Issue and PR templates
├── docs/                 # Detailed architecture & API documentation
├── site/                 # Interactive GitHub Pages documentation app
├── examples/             # Standalone runnable example programs
├── src/
│   ├── lib.rs            # Public API exports & prelude
│   ├── db.rs             # Top-level Vectrust instance coordinator
│   ├── collection.rs     # Thread-safe collection reader/writer state
│   ├── metric.rs         # SIMD-vectorized distance functions (L2, Cosine, Dot, L1)
│   ├── filter.rs         # JSON metadata predicate evaluation engine
│   ├── types.rs          # Core data models: Point, PointId, SearchQuery, Config
│   ├── error.rs          # Error types (VectrustError, Result)
│   ├── bin/
│   │   └── vectrust.rs   # CLI executable tool (info, check, benchmark, repl)
│   ├── index/
│   │   ├── hnsw.rs       # HNSW proximity graph ANN index
│   │   └── flat.rs       # Flat brute-force Rayon linear scan index
│   └── storage/
│       ├── wal.rs        # Write-Ahead Log append journal & CRC32
│       └── snapshot.rs   # Atomic snapshot serializer & CRC32 verification
└── tests/                # Integration test suites
```

---

## 9. Troubleshooting Common Issues

### Issue: `unsupported target` or missing SIMD optimization
- **Fix**: Vectrust uses safe, auto-vectorizable Rust loops with 8-way unrolling. Ensure you pass `--release` when benchmarking to enable LLVM autovectorization.

### Issue: Permission denied during git push
- **Fix**: Ensure your SSH command uses your Techwasti key:
  ```bash
  git config core.sshCommand "ssh -i ~/.ssh/id_rsa_techwasti"
  ```
