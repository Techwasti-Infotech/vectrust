## 📋 Summary of Changes

A concise description of the purpose of this PR and what was changed.

- Closes # (issue number, if applicable)

## 🔍 Type of Change

- [ ] 🐛 Bug fix (non-breaking change which fixes an issue)
- [ ] 🚀 New feature (non-breaking change which adds functionality)
- [ ] ⚡ Performance optimization (latency, throughput, or memory improvement)
- [ ] 💥 Breaking change (fix or feature that would cause existing functionality to not work as expected)
- [ ] 📝 Documentation update only
- [ ] 🧪 Tests or benchmarking additions

## 🧪 Testing Performed

Describe the tests you ran to verify your changes:
- [ ] `cargo test --all-targets --all-features` passed without errors
- [ ] `cargo fmt --check` passed
- [ ] `cargo clippy --all-targets --all-features -- -D warnings` passed
- [ ] Added new unit or integration tests covering edge cases
- [ ] Benchmarks measured (if modifying distance metrics or indexing)

## 📊 Benchmark Results (if applicable)

```text
Include any before/after benchmark results from `cargo run --release --example hnsw_vs_flat_benchmark`
```

## ✅ Checklist

- [ ] My code adheres to the project's [Coding Standards](CONTRIBUTING.md).
- [ ] I have updated the documentation in `docs/` and `README.md` if relevant.
- [ ] All public functions, structs, and traits have docstrings with examples.
- [ ] Commit messages follow the [Conventional Commits](CONTRIBUTING.md#step-4-commit-message-conventions) standard.
