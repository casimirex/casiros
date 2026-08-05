# Development

## Prerequisites

- Rust 1.85+ (stable)
- PostgreSQL 16+ (for integration tests)
- Docker (optional, for containerized development)

## Setup

```bash
# Clone the repository
git clone https://github.com/casimirex/casiros.git
cd casiros

# Start PostgreSQL for tests
docker compose up -d postgres

# Run the full test suite
CASIROS_POSTGRES__URL=postgresql://casiros:casiros@localhost:5432/casiros \
  cargo test --workspace --all-features
```

## Project Structure

```
casiros/
├── crates/
│   ├── core/          # Domain layer — pure financial formulas
│   ├── dag/           # Application layer — DAG engine, traits
│   ├── simulator/     # Application layer — Monte Carlo runner
│   ├── api/           # Infrastructure layer — HTTP server
│   ├── api-client/    # Typed Rust HTTP client
│   ├── cli/           # Command-line interface
│   ├── bench/         # Criterion benchmarks
│   ├── macros/        # Procedural macros
│   └── worker/        # Background job processor
├── python/            # Python SDK
├── web/               # Web dashboard
├── migrations/        # SQLx database migrations
├── config/            # Default configuration
└── docs/              # Documentation site
```

## Code Quality Gates

Every commit must pass:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo doc --workspace --no-deps --document-private-items
```

### Additional Checks

```bash
# Security audit
cargo audit

# License and dependency check
cargo deny check

# Code coverage (requires tarpaulin)
cargo tarpaulin --workspace --timeout 300 --fail-under 60
```

## Adding a Formula

1. Add the function to the appropriate module in `crates/core/src/`.
2. Include doc-tests with at least 2 assertions.
3. Add the formula kind to `crates/dag/src/graph.rs`.
4. Add evaluation logic in `evaluate_formula`.
5. Add the API request model in `crates/api/src/engine_builder.rs`.
6. Add the formula variant to `FormulaRequest` in `crates/api/src/models.rs`.
7. Run the full test suite.

## Running Benchmarks

```bash
cargo bench -p casiros-bench
```

Available benchmarks:
- DAG evaluation throughput
- Monte Carlo simulation (10k universes)
- JSON deserialization
- WebSocket streaming

## Python SDK

```bash
cd python
pip install -e .
pip install pytest responses
python -m pytest tests/
```

## Release Process

1. Update `CHANGELOG.md` with the new version.
2. Bump the version in `Cargo.toml` (`[workspace.package] version`).
3. Update the version in `crates/api/src/openapi.rs`.
4. Commit and tag: `git tag v0.X.0 && git push origin v0.X.0`.
5. The CI release workflow builds binaries and creates a GitHub Release.

## CI/CD

The CI pipeline runs on every push to `main` and pull request:

| Job | What it does |
|---|---|
| `lint` | `cargo fmt`, `cargo clippy`, `cargo check` |
| `test` | `cargo test` with PostgreSQL service container |
| `docs` | `cargo doc` |
| `audit` | `cargo audit` for security vulnerabilities |
| `deny` | `cargo-deny` for license and dependency checks |
| `coverage` | `cargo tarpaulin` with 60% line coverage gate |

The release workflow builds binaries for Linux, macOS, and Windows,
and attaches them to the GitHub Release.
