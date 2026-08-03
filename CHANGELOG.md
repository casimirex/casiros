# Changelog

All notable changes to the CASIROS project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.0] - 2026-08-03

### Added

- **OpenAPI 3.1 + Swagger UI** (`crates/api/src/openapi.rs`)
  - REST endpoints annotated with `utoipa`.
  - `/openapi.json` spec served automatically.
  - `/swagger-ui/*` interactive documentation.
  - `casiros-api-export-openapi` binary for static spec generation.

- **Typed Rust API client** (`crates/api-client/`)
  - `CasirosClient` built on `reqwest`.
  - Re-exports shared request/response models from `casiros_api`.
  - Committed `casiros.openapi.json` is the single source of truth.

- **API key authentication + per-client rate limiting** (`crates/api/src/auth.rs`)
  - `CASIROS_API_KEYS` comma-separated key list.
  - `CASIROS_RATE_LIMIT_RPM` sliding-window limit per key.
  - Supports `Authorization: Bearer <key>` and `X-API-Key: <key>`.
  - Public paths (`/healthz`, `/openapi.json`, `/swagger-ui/*`) bypass auth.

- **DAG snapshot persistence** (`crates/dag/src/persistence.rs`)
  - `EngineSnapshot` with name-based nodes and edges.
  - `CausalityEngine::to_snapshot` / `from_snapshot`.
  - JSON round-trip tests preserve structure and evaluation results.

- **Command-line interface** (`crates/cli/`)
  - `casiros-cli` binary with `evaluate`, `simulate`, `validate`, `save`, `load`.
  - Reads JSON graph/simulation requests and writes JSON results.

- **Criterion benchmark suite** (`crates/bench/`)
  - `dag_evaluate`: chained graph evaluation throughput.
  - `simulator_run`: Monte Carlo simulation throughput.
  - `api_deserialize`: request deserialization throughput.

- **New financial formulas** (`casiros_core`)
  - `options::black_scholes_call` / `black_scholes_put`.
  - `general::amortization_payment` / `amortization_schedule`.
  - `stocks_bonds::yield_to_maturity_approximation`.
  - `markets::simple_moving_average`.
  - Formula count: 23 → 28.

### Changed

- README updated with OpenAPI/Swagger URLs, CLI usage, auth env vars, and expanded formula catalog.

## [0.1.0] - 2026-08-01

### Added

- Initial MVP scaffold and workspace layout.
- `casiros_core`: 23 pure, stateless financial formulas across general, ratios, banking, markets, stocks & bonds, and corporate domains.
- `casiros_dag`: Causality graph engine with topological evaluation, cycle detection, and depth limits.
- `casiros_simulator`: Monte Carlo multiverse engine with uniform/fixed distributions and reproducible seeds.
- `casiros_api`: Actix-Web REST API with `/healthz`, `/evaluate`, `/simulate`, and hard resource limits.
- `casiros_macros`: `#[derive(Narrative)]` procedural macro for human-readable struct summaries.
- NASA/JPL-grade linting: `#![forbid(unsafe_code)]`, `#![deny(missing_docs)]`, `#![deny(clippy::pedantic)]`, `#![deny(warnings)]`.
- 100% doc-test coverage on all public functions.
- Docker Compose and plain Docker support.

[Unreleased]: https://github.com/casimirex/casiros/compare/v0.2.0...HEAD
[0.2.0]: https://github.com/casimirex/casiros/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/casimirex/casiros/releases/tag/v0.1.0
