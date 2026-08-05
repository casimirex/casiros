# Changelog

All notable changes to the CASIROS project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.4.0] - 2026-08-05

### Added

- **Prometheus metrics** (Phase 6):
  - `GET /metrics` endpoint returning Prometheus text format (public, no auth).
  - Request count and duration histograms by method, path, and status.
  - Rate-limit denial counter by tenant.
  - Job state transition counter by status.
  - Audit write failure counter.

- **Per-key rate limit tiers**:
  - `CASIROS_API_KEY_TENANTS` now supports an optional 4th field for per-key
    RPM: `key:tenant:workspace:rpm`.
  - Falls back to the global `CASIROS_RATE_LIMIT_RPM` when no per-key limit
    is configured.

- **Redis-backed FormulaCache**:
  - `RedisFormulaCache` implements the `FormulaCache` trait using Redis
    SETEX/GET with configurable TTL.
  - Feature-gated behind the `redis` feature flag.

- **Stateful simulation runner**:
  - Worker now processes universes in batches of 100 with progress
    checkpointing after each batch.
  - Checks for cancellation before each batch for graceful stop.
  - Results are aggregated via `MonteCarloConfig::aggregate`.

- **Admin API**:
  - `GET /admin/tenants` — list tenants.
  - `POST /admin/tenants` — provision a new tenant.
  - `GET /admin/tenants/{id}/stats` — usage statistics.
  - `POST /admin/keys` — generate a new API key.
  - `POST /admin/keys/{id}/revoke` — revoke an API key.
  - All admin endpoints protected by `CASIROS_ADMIN_KEY`.

## [0.3.0] - 2026-08-05

### Added

- **Tenant isolation and workspace scoping** (Phase 5):
  - `TenantId`, `WorkspaceId`, and `Principal` value objects in the domain layer.
  - `TenantResolver` trait mapping API keys to tenants via `CASIROS_API_KEY_TENANTS`.
  - All snapshots, audit events, and simulation jobs scoped to a tenant/workspace.
  - Cross-tenant access rejected at the storage layer.

- **Immutable audit trail**:
  - `AuditEvent`, `AuditAction`, and `AuditResult` domain types.
  - `AuditLog` trait with append-only record and tenant-scoped list.
  - In-memory and PostgreSQL backends behind an `AuditSink` wrapper.
  - Middleware records one event per authenticated request.
  - `GET /audit` returns the calling tenant's events, newest first.

- **Async simulation jobs**:
  - `JobId`, `JobStatus`, `JobProgress` domain types with full lifecycle.
  - `JobStore` trait with enqueue, claim, progress, complete, fail, cancel.
  - In-memory and PostgreSQL (`FOR UPDATE SKIP LOCKED`) backends.
  - `POST /simulate/jobs`, `GET /simulate/jobs/{id}`, `POST /simulate/jobs/{id}/cancel`.
  - `GET /ws/jobs/{id}` WebSocket streaming progress frames every 500ms.

- **Background worker** (`casiros-worker`):
  - Standalone binary that claims and executes queued simulation jobs.
  - Multiple workers can run concurrently without double-claiming.

- **DAG result cache**:
  - `FormulaCache` trait with `CacheKey` for deterministic memoization.
  - In-memory implementation for single-process deployments.

- **Client updates**:
  - `create_job`, `get_job`, `cancel_job` methods on `CasirosClient`.
  - Job request/response models re-exported from the API crate.

### Fixed

- Migration 0004 declared `result_snapshot_id UUID` referencing `snapshots.id`
  (TEXT); the foreign key could not be created.
- Migration 0002 added foreign keys from `snapshots` to `tenants` and
  `workspaces` without seeding the default tenant/workspace rows.
- Postgres test fixtures now panic on migration failure instead of silently
  skipping, so a broken schema can never masquerade as a passing suite.

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

- **Persistent snapshot REST endpoints** (`crates/api/src/snapshot_handlers.rs`)
  - `POST /snapshots`, `GET /snapshots`, `GET /snapshots/{id}`, `DELETE /snapshots/{id}`.
  - Backend-agnostic `SnapshotRepository` trait in `casiros_dag` with an in-memory implementation.
  - `SnapshotRepo` wrapper in `casiros_api` for clean infrastructure wiring.

- **Advanced option pricing & Greeks** (`casiros_core`)
  - `options::binomial_option_call` / `binomial_option_put` (Cox-Ross-Rubinstein).
  - `options::black_scholes_delta`, `black_scholes_gamma`, `black_scholes_vega`, `black_scholes_theta`, `black_scholes_rho`.
  - Wired through `casiros_dag::FormulaKind`, `casiros_api::FormulaRequest`, engine builder, and CLI reverse mapper.
  - Formula count: 28 → 35.

- **CSV/Excel import-export** (`crates/cli/src/convert.rs`)
  - `casiros-cli convert input.{csv,xlsx,json} output.{csv,xlsx,json}`.
  - Imports CSV/Excel into JSON input maps; exports `EvaluateResponse` and `SimulateResponse` to CSV/Excel.

- **Python client SDK** (`python/`)
  - Synchronous `CasirosClient` with typed models for evaluate, simulate, and snapshot endpoints.
  - Bearer-token API key support and `responses`-based pytest suite.

- **Web dashboard** (`web/`)
  - Static single-page dashboard for evaluate/simulate/snapshot operations with live Chart.js visuals.
  - Served by the API at `/dashboard` with permissive CORS for local development.

- **Streaming simulation progress** (`crates/api/src/streaming_handlers.rs`)
  - `POST /simulate/stream` returns `text/event-stream` with progress and final result frames.

- **Performance & release engineering**
  - Monte Carlo universes already run in parallel via `rayon`.
  - CI hardening: `cargo audit`, `cargo deny`, `cargo tarpaulin`, cross-platform release workflow.
  - `deny.toml`, `LICENSE-MIT`, `LICENSE-APACHE`, and crates.io-ready workspace metadata.

- **New financial formulas** (`casiros_core`)
  - `options::black_scholes_call` / `black_scholes_put`.
  - `general::amortization_payment` / `amortization_schedule`.
  - `stocks_bonds::yield_to_maturity_approximation`.
  - `markets::simple_moving_average`.
  - Formula count: 23 → 28.

### Changed

- README updated with dashboard, streaming, Python SDK, CSV/Excel CLI, advanced options, snapshot, and OpenAPI/Swagger usage.

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
