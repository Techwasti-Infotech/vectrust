# Contributing to Vectrust

Thank you for your interest in contributing to **Vectrust**! We welcome contributions from everyone, whether it is reporting a bug, proposing new features, improving documentation, or submitting performance optimizations.

---

## 🌟 Code of Conduct

We are committed to providing a welcoming, inclusive, and harassment-free environment for all contributors. Please treat everyone with respect and kindness.

---

## 🚀 How to Contribute

### 1. Reporting Bugs
- Before filing a new issue, search existing [GitHub Issues](https://github.com/techwastiinfotech/vectrust/issues) to avoid duplicates.
- Use our [Bug Report Template](.github/ISSUE_TEMPLATE/bug_report.yml).
- Include minimal reproducible code examples, vector dimensions, database configurations, and environment details (OS, Rust version).

### 2. Suggesting Features & Enhancements
- Open a feature discussion using our [Feature Request Template](.github/ISSUE_TEMPLATE/feature_request.yml).
- Explain the motivation, use cases, and proposed API design.
- For architectural changes (e.g., new index types, quantization, SIMD intrinsics), discuss the design with maintainers before writing extensive code.

### 3. Improving Documentation
- Documentation fixes, typo corrections, and code sample additions are always welcome!
- Documentation files are located in `docs/` and `site/`.

---

## 🛠️ Development Workflow

### Step 1: Fork and Clone

1. Fork the repository on GitHub: `https://github.com/techwastiinfotech/vectrust`
2. Clone your fork locally:
   ```bash
   git clone https://github.com/<your-username>/vectrust.git
   cd vectrust
   ```
3. Add the upstream remote:
   ```bash
   git remote add upstream https://github.com/techwastiinfotech/vectrust.git
   ```

### Step 2: Create a Feature Branch

Create a branch with a descriptive name:

```bash
git checkout -b feat/quantized-product-index
# or
git checkout -b fix/wal-truncate-edge-case
```

### Step 3: Make Your Changes & Test

Follow the [Local Setup Guide](docs/local_setup.md) to set up your environment:

```bash
# Verify compilation
cargo check --all-targets --all-features

# Run the full test suite
cargo test --all-targets --all-features

# Format code
cargo fmt

# Run linter
cargo clippy --all-targets --all-features -- -D warnings
```

If your change modifies vector distance calculations or index traversal, run benchmarks to verify performance has not regressed:

```bash
cargo run --release --example hnsw_vs_flat_benchmark
```

### Step 4: Commit Message Conventions

We use **Conventional Commits** to keep the git history clean and automate changelog generation:

| Prefix | Description | Example |
| :--- | :--- | :--- |
| `feat:` | A new feature or capability | `feat: implement scalar quantization SQ8 for HNSW` |
| `fix:` | A bug fix | `fix: handle empty collection search gracefully` |
| `perf:` | A performance optimization | `perf: unroll AVX2 dot-product accumulator loop` |
| `docs:` | Documentation changes only | `docs: add metadata filtering examples in README` |
| `test:` | Adding or improving tests | `test: add fuzz tests for WAL crash recovery` |
| `refactor:`| Code change that neither fixes a bug nor adds a feature | `refactor: extract graph layer iterator` |
| `chore:` | Build process, dependencies, or toolchain updates | `chore: update rayon dependency to 1.10` |

### Step 5: Submitting a Pull Request

1. Push your branch to your fork:
   ```bash
   git push origin feat/your-feature-name
   ```
2. Open a Pull Request against the `main` branch of `techwastiinfotech/vectrust`.
3. Complete the [Pull Request Template](.github/PULL_REQUEST_TEMPLATE.md).
4. Ensure all CI checks pass (Linux, Windows, macOS).

---

## 📐 Coding Standards & Guidelines

1. **Pure Rust**: Maintain zero C/C++ dependencies. The library must compile cleanly using standard `cargo build`.
2. **Safety First**: Avoid `unsafe` code unless required for verified SIMD performance, and always provide an explanatory `// SAFETY:` comment.
3. **Public API Documentation**: All public items (`pub struct`, `pub fn`, `pub trait`, `pub enum`) must have comprehensive docstrings with examples:
   ```rust
   /// Searches the collection for nearest neighbors to the query vector.
   ///
   /// # Examples
   /// ```rust
   /// use vectrust::prelude::*;
   /// # let col = Collection::in_memory(CollectionConfig::builder(4, Metric::Cosine).build());
   /// let results = col.search(SearchQuery::new(vec![0.1, 0.2, 0.3, 0.4], 5))?;
   /// # Ok::<(), vectrust::VectrustError>(())
   /// ```
   ```
4. **Non-Poisoning Locks**: Always use `parking_lot::RwLock` or `parking_lot::Mutex` instead of `std::sync` primitives to prevent lock poisoning on panic.
5. **Deterministic Error Handling**: Use `VectrustError` with informative error variants instead of panicking or unwrap in library code.

---

## 🤝 Community & Help

- GitHub Discussions: [Ask Questions](https://github.com/techwastiinfotech/vectrust/discussions)
- GitHub Issues: [Issue Tracker](https://github.com/techwastiinfotech/vectrust/issues)
- Maintainer Contact: `techwastiinfotech@users.noreply.github.com`
