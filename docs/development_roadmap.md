# CASIROS Development Roadmap — Future Improvements

This document captures planned improvements and feature ideas for future
development phases. Items are organized by theme and rough priority.

---

## Infrastructure Hardening

### Distributed Rate Limiting with Redis

The current `RateLimiter` is per-process in-memory, using a sliding window
of timestamps behind `Arc<Mutex<HashMap>>`. This means each API instance
has its own rate limit bucket, so running 3 instances gives 3× the effective
limit. A Redis-backed rate limiter would share state across instances.

**Approach:**
- Add a `RedisRateLimiter` implementation using Redis sorted sets or the
  `INCR`/`EXPIRE` pattern for sliding window rate limiting.
- The existing `RateLimiter` trait (or a new trait) would have in-memory
  and Redis implementations, selected by configuration.
- Feature-gate behind `redis` feature flag.

**Files to modify:**
- `crates/api/src/auth.rs` — add `RedisRateLimiter`, update `auth_middleware`
- `crates/api/src/main.rs` — wire Redis rate limiter at startup
- `config/default.toml` — add `[rate_limiter]` section
- `docs/ops.md` — document Redis rate limiting

### Kubernetes / Helm Charts

Production deployment manifests for Kubernetes, including:
- Deployment with horizontal pod autoscaling
- Service and Ingress configuration
- ConfigMap and Secret management
- PersistentVolumeClaim for Postgres
- Network policies
- PodDisruptionBudget

**Approach:**
- Create a `deploy/helm/casiros/` directory with Helm chart structure.
- Support configurable replica counts, resource limits, and environment
  variables.
- Include a values.yaml with sensible defaults for development and
  production profiles.

**Files to create:**
- `deploy/helm/casiros/Chart.yaml`
- `deploy/helm/casiros/values.yaml`
- `deploy/helm/casiros/templates/deployment-api.yaml`
- `deploy/helm/casiros/templates/deployment-worker.yaml`
- `deploy/helm/casiros/templates/service.yaml`
- `deploy/helm/casiros/templates/ingress.yaml`
- `deploy/helm/casiros/templates/configmap.yaml`
- `deploy/helm/casiros/templates/secret.yaml`
- `deploy/helm/casiros/templates/hpa.yaml`
- `deploy/helm/casiros/templates/_helpers.tpl`

### Load Testing with k6

Add a load testing suite using k6 to validate capacity planning and
identify bottlenecks.

**Approach:**
- Create a `load-tests/` directory with k6 scripts.
- Test scenarios: health check flood, DAG evaluation, Monte Carlo
  simulation, async job enqueue and poll, mixed workload.
- Measure p50/p95/p99 latency, throughput, and error rate.
- Integrate into CI as a non-blocking performance regression check.

**Files to create:**
- `load-tests/scenarios/health.js`
- `load-tests/scenarios/evaluate.js`
- `load-tests/scenarios/simulate.js`
- `load-tests/scenarios/jobs.js`
- `load-tests/scenarios/mixed.js`
- `load-tests/run.sh`

---

## Feature Work

### WebSocket Improvements

The existing `/ws/jobs/{id}` and `/ws/simulate` endpoints are functional
but lack production features.

**Improvements:**
- **Heartbeat/ping-pong** — send periodic pings to detect dead connections.
- **Reconnection support** — include a `sequence_number` in each frame so
  clients can detect missed messages and reconnect.
- **Compression** — enable `permessage-deflate` extension for large result
  payloads.
- **Authentication** — support API key authentication via query parameter
  or first-frame token for WebSocket connections.
- **Rate limiting** — apply rate limiting to WebSocket message frequency.

**Files to modify:**
- `crates/api/src/websocket_handlers.rs` — add heartbeat, compression
- `crates/api/src/job_ws_handlers.rs` — add heartbeat, sequence numbers
- `crates/api/src/auth.rs` — add WebSocket auth support

### Documentation Site Content

The MkDocs site exists with 6 pages but could benefit from richer content.

**Improvements:**
- **Tutorials** — step-by-step guides for common workflows (e.g., "Build a
  DCF valuation model", "Run a Monte Carlo simulation from Python").
- **Formula catalog** — auto-generated reference page listing all 55+
  formulas with descriptions, parameters, and examples.
- **SDK reference** — auto-generated Python SDK reference using mkdocstrings.
- **FAQ** — common questions and troubleshooting.
- **Video / interactive demos** — embedded examples or links to screencasts.

**Files to modify:**
- `docs/tutorials/dcf-valuation.md`
- `docs/tutorials/python-sdk.md`
- `docs/formulas.md`
- `docs/faq.md`
- `mkdocs.yml` — update nav

### API Client Improvements

The Rust and Python API clients could be enhanced.

**Improvements:**
- **Async Python client** — add an async variant using `httpx` or `aiohttp`.
- **Retry with backoff** — add automatic retry for 429 (rate limit) and
  5xx responses.
- **Streaming support** — add WebSocket client support for job progress
  streaming.
- **Connection pooling** — reuse HTTP connections in the Python client.
- **Type hints** — improve Python type annotations for better IDE support.

**Files to modify:**
- `python/casiros/client.py` — add async client, retry, WebSocket
- `crates/api-client/src/lib.rs` — add retry, WebSocket

---

## Maintenance

### Dependency Updates

Review and update stale dependencies.

**Areas to check:**
- `cargo outdated` — identify outdated crates.
- `cargo audit` — check for security vulnerabilities.
- `cargo deny` — verify license compliance.
- Python dependencies — update `pyproject.toml` pins.

### Security Review

Another security pass now that the codebase has grown significantly.

**Areas to review:**
- Input validation — ensure all API endpoints validate input size and shape.
- CORS configuration — review permissive CORS for production.
- API key storage — consider hashing API keys at rest.
- Rate limit bypass — verify rate limiting cannot be bypassed via
  WebSocket or streaming endpoints.
- Dependency vulnerabilities — run `cargo audit` and address findings.

### Performance Optimization

Profile and optimize hot paths.

**Areas to investigate:**
- DAG evaluation — profile `evaluate_formula` for the most common formulas.
- Monte Carlo simulation — profile `run_batch` for parallel efficiency.
- JSON serialization — profile `serde_json` for large request/response bodies.
- Database queries — profile Postgres queries for audit and job listing.
- Memory usage — profile the worker for long-running simulations.

---

## Testing Strategy

### End-to-end smoke tests (added)

Six defects reached the repository despite a green test suite, all for the same
structural reason: every suite rebuilt the Actix app in-process and so never
executed `main.rs`. The `casiros-e2e` crate closes that gap by launching the
real binaries, and `scripts/browser-smoke.js` covers the dashboard.

### Remaining gaps

- **Docker image smoke test** — build the image and assert the container
  answers on its published port. The `Dockerfile` bind-address defect would
  have been caught by exactly this.
- **Upgrade test** — run migrations from an older schema against a current
  binary, so a migration that breaks existing data is caught before release.
- **Multi-replica test** — two API instances against one database, asserting
  rate limits and job claiming behave sanely. This would quantify the
  per-process rate-limit caveat rather than leaving it a footnote.

## Formula Surface

As of v0.9.0 every core formula is reachable from the API — 63 of 63 — and the
Formula Reference documents all 62 that are callable through `/evaluate`. The
63rd, `amortization_schedule`, has its own endpoint for the reason below.

### `amortization_schedule` has an endpoint (done in 0.9.0)

`POST /schedule/amortization` returns the repayment table as rows, alongside
the level payment and total interest. Every core formula now has a route:
63 of 63.

It could not be a `FormulaKind` variant because a DAG node evaluates to
exactly one `Decimal`, and this returns `Vec<AmortizationPeriod>`. Flattening
the table into a single number would have discarded the point of it, and
widening every node's output type for one formula would have been worse.

Still open: the CLI has CSV and Excel export paths that are table-shaped
already, and a repayment schedule is the natural thing to export. Nothing
wires the endpoint to them yet.

### Formula Reference is complete (done in 0.8.0)

`docs/formulas.html` documents all 62 exposed formulas, each with an
explanation, its mathematics, a worked example, and a dashboard screenshot
captured against a live server.

Screenshots are now reproducible rather than hand-made:

```bash
cargo run -p casiros-api &
npm install puppeteer-core@23 --no-save
node scripts/capture-formula-screenshots.js http://localhost:8080 <api-key>
```

Pass formula names as trailing arguments to refresh only those. The script
fails rather than writing an image if the server returns an error, so a
screenshot in this reference cannot show a request that did not work.

## Version History

| Version | Date | Highlights |
|---|---|---|
| v0.9.0 | 2026-08-06 | `POST /schedule/amortization`; every core formula now has a route, 63 of 63 |
| v0.8.0 | 2026-08-06 | 17 remaining formulas wired through DAG/API/CLI (NPV, IRR, Sharpe, bond price); API now exposes 62 of 63 |
| v0.7.0 | 2026-08-05 | Formulas wired through API, cache in evaluator, benchmarks |
| v0.6.0 | 2026-08-05 | OTel, formulas, docs site, API versioning |
| v0.5.0 | 2026-08-05 | Python SDK, dashboard, formulas, CI/CD, security audit |
| v0.4.0 | 2026-08-05 | Metrics, rate-limit tiers, Redis cache, stateful worker, admin API |
| v0.3.0 | 2026-08-05 | Tenant isolation, audit trail, async jobs, background worker |
| v0.2.0 | 2026-08-03 | OpenAPI, API client, CLI, benchmarks, dashboard, Python SDK |
| v0.1.0 | 2026-08-01 | Initial MVP with core formulas and DAG engine |
