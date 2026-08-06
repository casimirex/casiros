# CASIROS

**CASIROS** is a NASA/JPL-grade Financial Physics Engine & Multiverse Simulator written in Rust.

## Mission

Every financial formula implemented as a pure, stateless, provably correct function.
Every dependency traced through a causality graph. Every scenario simulated in parallel.

## Standards

- `#![forbid(unsafe_code)]` — Memory safety is absolute.
- `#![deny(missing_docs)]` — Undocumented code does not compile.
- `#![deny(clippy::pedantic)]` — Every Clippy lint is a hard error.
- `rust_decimal::Decimal` — Floating-point math is banned for money and ratios.
- 100% doc-test coverage on all public functions.

## Workspace

| Crate | Layer | Purpose | Status |
|---|---|---|---|
| `crates/core` | Domain | Pure financial formulas and shared types | ✅ Implemented (44 formulas) |
| `crates/dag` | Application | Causality graph engine | ✅ Implemented |
| `crates/simulator` | Application | Monte Carlo multiverse engine | ✅ Implemented |
| `crates/api` | Infrastructure | Actix-Web REST interface | ✅ Implemented |
| `crates/api-client` | Infrastructure | Typed Rust client generated from the OpenAPI contract | ✅ Implemented |
| `crates/macros` | Infrastructure | Procedural macros for narrative generation | ✅ Implemented |
| `crates/cli` | Infrastructure | `casiros-cli` command-line tool | ✅ Implemented |
| `crates/bench` | Infrastructure | Criterion benchmark suite | ✅ Implemented |
| `python/` | SDK | Synchronous Python client | ✅ Implemented |
| `web/` | Frontend | Static dashboard for evaluate/simulate/snapshot | ✅ Implemented |

## Quick Start

```bash
# Run all tests (doc-tests + integration tests across all crates)
cargo test --workspace

# Run strict Clippy
cargo clippy --workspace --all-targets --all-features

# Build documentation
cargo doc --no-deps --workspace

# Start the API server locally
cargo run -p casiros-api

# Export the OpenAPI contract
cargo run -p casiros-api --bin casiros-api-export-openapi > casiros.openapi.json

# Use the CLI
cargo run -p casiros-cli -- evaluate request.json
cargo run -p casiros-cli -- simulate request.json
cargo run -p casiros-cli -- validate request.json
cargo run -p casiros-cli -- save engine.json snapshot.json
cargo run -p casiros-cli -- load snapshot.json engine.json

# Convert between JSON, CSV, and Excel
cargo run -p casiros-cli -- convert inputs.csv inputs.json
cargo run -p casiros-cli -- convert response.json response.csv
cargo run -p casiros-cli -- convert response.json response.xlsx

# Open the dashboard
cargo run -p casiros-api
# -> http://localhost:8080/dashboard

# Use the Python client
cd python && pip install -e ".[dev]" && pytest
```

## API Endpoints

The API server binds to `127.0.0.1:8080` by default (override with `CASIROS__BIND_ADDR`).

### `GET /healthz`

Liveness/readiness probe.

```bash
curl http://localhost:8080/healthz
```

Response:

```json
{ "status": "ok" }
```

### `POST /evaluate`

Evaluates a causality graph with fixed inputs and returns every node's computed value.

```bash
curl -X POST http://localhost:8080/evaluate \
  -H "Content-Type: application/json" \
  -d '{
    "nodes": [
      { "input": { "name": "principal" } },
      { "input": { "name": "rate" } },
      { "formula": {
        "name": "fv",
        "kind": {
          "formula": "future_value",
          "present_value": { "node": "principal" },
          "rate": { "node": "rate" },
          "periods": 10
        }
      }}
    ],
    "edges": [
      { "dependency": "principal", "dependent": "fv" },
      { "dependency": "rate", "dependent": "fv" }
    ],
    "inputs": {
      "principal": "100.0",
      "rate": "0.05"
    }
  }'
```

Response:

```json
{
  "outputs": {
    "principal": "100.0",
    "rate": "0.05",
    "fv": "162.88946267774721"
  }
}
```

### `POST /simulate`

Runs a Monte Carlo simulation by perturbing input nodes with distributions and
aggregating a target node's output.

```bash
curl -X POST http://localhost:8080/simulate \
  -H "Content-Type: application/json" \
  -d '{
    "nodes": [
      { "input": { "name": "principal" } },
      { "input": { "name": "rate" } },
      { "formula": {
        "name": "fv",
        "kind": {
          "formula": "future_value",
          "present_value": { "node": "principal" },
          "rate": { "node": "rate" },
          "periods": 1
        }
      }}
    ],
    "edges": [
      { "dependency": "principal", "dependent": "fv" },
      { "dependency": "rate", "dependent": "fv" }
    ],
    "bindings": [
      { "node": "principal", "distribution": { "kind": "uniform", "low": 90.0, "high": 110.0 } },
      { "node": "rate", "distribution": { "kind": "fixed", "value": 0.05 } }
    ],
    "target": "fv",
    "universe_count": 1000,
    "seed": 42
  }'
```

Response:

```json
{
  "count": 1000,
  "mean": "105.003",
  "median": "104.987",
  "min": "94.50",
  "max": "115.50"
}
```

### `POST /simulate/stream`

Streams simulation progress as `text/event-stream` and ends with the final aggregate result.

```bash
curl -N -X POST http://localhost:8080/simulate/stream \
  -H "Content-Type: application/json" \
  -d '{ "nodes": [...], "bindings": [...], "target": "fv", "universe_count": 1000 }'
```

Each line is an SSE `data:` frame:

```text
data: {"type":"progress","completed":100,"total":1000,"partial":{"count":100,...}}

data: {"type":"result","result":{"count":1000,"mean":"...",...}}
```

### `GET /ws/simulate`

WebSocket alternative to the SSE stream. Connect, send a single JSON `SimulateRequest` text frame,
and receive progress + result frames.

```javascript
const ws = new WebSocket('ws://localhost:8080/ws/simulate');
ws.onopen = () => {
  ws.send(JSON.stringify({
    nodes: [...],
    edges: [...],
    bindings: [...],
    target: 'fv',
    universe_count: 1000,
    seed: 42
  }));
};
ws.onmessage = (event) => {
  const msg = JSON.parse(event.data);
  console.log(msg.type, msg);
};
```

### Snapshot endpoints

Persist and retrieve engine graphs.

- `POST /snapshots` — save a snapshot (`{ "id", "nodes", "edges" }`).
- `GET /snapshots` — list stored snapshots.
- `GET /snapshots/{id}` — load a snapshot.
- `DELETE /snapshots/{id}` — delete a snapshot.

## Web Dashboard

The API serves a static dashboard at `/dashboard`:

```bash
cargo run -p casiros-api
```

Open `http://localhost:8080/dashboard`. It supports evaluate, simulate, snapshot management,
and live bar charts via Chart.js.

## CASIROS Python Client SDK

A synchronous Python client lives in `python/`.

```bash
cd python
pip install -e ".[dev]"
pytest
```

```python
from casiros import CasirosClient, EvaluateRequest

client = CasirosClient("http://localhost:8080", api_key="secret")
print(client.healthz())
response = client.evaluate(EvaluateRequest(nodes=..., edges=..., inputs=...))
print(response.outputs)
```

## OpenAPI & Swagger UI

The REST API is fully documented with an `OpenAPI 3.1` contract generated by `utoipa`.

- JSON spec: `GET /openapi.json`
- Interactive docs: `http://localhost:8080/swagger-ui`

The generated `casiros.openapi.json` is committed under `crates/api-client/` and
powers the typed `casiros-api-client` crate.

## Configuration

Configuration is layered from `config/default.toml` (embedded fallback), an optional
`config/default.toml` at runtime, and environment variables prefixed with `CASIROS_`.
Nested keys use double underscores, e.g. `CASIROS__SNAPSHOT__BACKEND` overrides `[snapshot].backend`.

| Variable | Default | Purpose |
|---|---|---|
| `CASIROS__BIND_ADDR` | `127.0.0.1:8080` | HTTP server bind address |
| `CASIROS__LOG_LEVEL` | `info` | Tracing filter (`info`, `debug`, etc.) |
| `CASIROS_RATE_LIMIT_RPM` | `60` | Per-API-key rate limit |
| `CASIROS__SNAPSHOT__BACKEND` | `memory` | Snapshot backend: `memory` or `postgres` |
| `CASIROS__POSTGRES__URL` | `postgresql://casiros:casiros@localhost:5432/casiros` | PostgreSQL connection URL |

```bash
CASIROS__BIND_ADDR=0.0.0.0:3000 \
CASIROS__SNAPSHOT__BACKEND=postgres \
CASIROS__POSTGRES__URL=postgresql://user:pass@localhost:5432/casiros \
  cargo run -p casiros-api
```

## Authentication & Rate Limiting

Protected endpoints require an API key via either:

- `Authorization: Bearer <key>`
- `X-API-Key: <key>`

Public paths (`/healthz`, `/openapi.json`, `/swagger-ui/*`, `/ws/simulate`) are always accessible.

Configuration:

- `CASIROS_API_KEYS` — comma-separated list of valid keys. If unset, authentication is disabled.
- `CASIROS_RATE_LIMIT_RPM` — requests per minute per key (default: `60`).

```bash
CASIROS_API_KEYS=prod-key-1,prod-key-2 CASIROS_RATE_LIMIT_RPM=120 cargo run -p casiros-api
```

## CASIROS CLI

The `casiros-cli` binary reads graph/simulation requests as JSON and writes
colorful JSON results to stdout.

```bash
# Evaluate a graph
casiros-cli evaluate request.json

# Run a simulation
casiros-cli simulate request.json

# Validate a graph without computing values
casiros-cli validate request.json

# Persist an engine file to a snapshot
casiros-cli save engine.json snapshot.json

# Restore an engine file from a snapshot
casiros-cli load snapshot.json engine.json

# Convert between JSON, CSV, and Excel
casiros-cli convert inputs.csv inputs.json
casiros-cli convert response.json response.csv
casiros-cli convert response.json response.xlsx
```

## Docker

Build and run with Docker Compose:

```bash
docker compose up --build
```

Or with plain Docker:

```bash
docker build -t casiros-api:latest .
docker run -p 8080:8080 -e CASIROS__BIND_ADDR=0.0.0.0:8080 casiros-api:latest
```

The runtime image uses a non-root user, exposes port `8080`, and includes a
health check against `/healthz`.

## Implemented Formulas

- **General**: Future Value, Present Value, Annuity FV/PV, Perpetuity PV, Growing Perpetuity PV, Effective Annual Rate, Amortization Payment, Amortization Schedule, Continuous Compounding FV
- **Financial Ratios**: ROE, ROA, ROI, DuPont ROE, Current Ratio, Quick Ratio, Debt-to-Equity, Profit Margin, Asset Turnover, Equity Multiplier, Interest Coverage, Inventory Turnover, Cash Conversion Cycle
- **Banking**: Net Interest Margin, Loan-to-Deposit Ratio, Capital Adequacy Ratio, Provision Coverage Ratio
- **Markets**: Sharpe Ratio, Jensen's Alpha, Simple Moving Average, Treynor Ratio, Value at Risk, Expected Shortfall
- **Stocks & Bonds**: Dividend Discount Model, Bond Price, Yield-to-Maturity Approximation, Discounted Cash Flow, Macaulay Duration, Modified Duration, Convexity
- **Corporate**: WACC, Free Cash Flow to Firm, Free Cash Flow to Equity, Sustainable Growth Rate, Internal Growth Rate, Economic Value Added
- **Options**: Black-Scholes Call/Put, Binomial Call/Put, Black-Scholes Delta, Gamma, Vega, Theta, Rho

## Narrative Macros

The `casiros-macros` crate provides a `#[derive(Narrative)]` procedural macro
that turns any struct with named fields into a human-readable sentence, useful
for audit trails and CFO memos.

```rust
use casiros_core::narrative::Narrative;
use casiros_macros::Narrative;
use rust_decimal_macros::dec;

#[derive(Narrative)]
#[narrative(prefix = "Capital Structure")]
struct CapitalStructure {
    equity: rust_decimal::Decimal,
    #[narrative(name = "total debt")]
    debt: rust_decimal::Decimal,
}

let cs = CapitalStructure { equity: dec!(600.0), debt: dec!(400.0) };
assert_eq!(cs.narrative(), "Capital Structure: equity = 600.00, total debt = 400.00");
```

Supported field attributes:

- `#[narrative(skip)]` — omit the field.
- `#[narrative(name = "...")]` — custom display name.
- `#[narrative(prefix = "...")]` on the struct — custom prefix (defaults to struct name).

## Security

- `.env` files are excluded from version control. Never commit credentials.
- The `core` crate forbids `unsafe` code and uses `rust_decimal` for all financial arithmetic.
- The API enforces hard limits on graph size and simulation scale.
- API keys and per-client rate limiting are available via `CASIROS_API_KEYS` and `CASIROS_RATE_LIMIT_RPM`.

## Security limits

To protect the server from accidental or malicious overload:

- Maximum nodes per graph: `100`
- Maximum edges per graph: `500`
- Maximum graph depth: `50`
- Maximum universes per simulation: `100_000`
- Maximum input bindings per simulation: `50`

Requests exceeding these limits return `400 Bad Request` with a descriptive error message.

## Roadmap

See [`CASIROS_ROADMAP.md`](CASIROS_ROADMAP.md) for the full architectural blueprint, NASA/JPL coding standard adaptation, Clean Architecture layers, and the complete formula catalog.
