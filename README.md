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
| `crates/core` | Domain | Pure financial formulas and shared types | ✅ Implemented (23 formulas) |
| `crates/dag` | Application | Causality graph engine | ✅ Implemented |
| `crates/simulator` | Application | Monte Carlo multiverse engine | ✅ Implemented |
| `crates/api` | Infrastructure | Actix-Web REST interface | ✅ Implemented |
| `crates/macros` | Infrastructure | Procedural macros for narrative generation | 🟡 Stub |

## Quick Start

```bash
# Run all tests (doc-tests + integration tests across all crates)
cargo test --workspace

# Run strict Clippy
cargo clippy --workspace --all-targets -- -D warnings

# Build documentation
cargo doc --no-deps --workspace

# Start the API server locally
cargo run -p casiros-api
```

## API Endpoints

The API server binds to `127.0.0.1:8080` by default (override with `CASIROS_BIND_ADDR`).

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

### Security limits

To protect the server from accidental or malicious overload:

- Maximum nodes per graph: `100`
- Maximum edges per graph: `500`
- Maximum graph depth: `50`
- Maximum universes per simulation: `100_000`
- Maximum input bindings per simulation: `50`

Requests exceeding these limits return `400 Bad Request` with a descriptive error message.

## Docker

Build and run with Docker Compose:

```bash
docker compose up --build
```

Or with plain Docker:

```bash
docker build -t casiros-api:latest .
docker run -p 8080:8080 -e CASIROS_BIND_ADDR=0.0.0.0:8080 casiros-api:latest
```

The runtime image uses a non-root user, exposes port `8080`, and includes a
health check against `/healthz`.

## Implemented Formulas

- **General**: Future Value, Present Value, Annuity FV/PV, Perpetuity PV, Effective Annual Rate
- **Financial Ratios**: ROE, ROA, DuPont ROE, Current Ratio, Debt-to-Equity
- **Banking**: Net Interest Margin, Loan-to-Deposit Ratio
- **Markets**: Sharpe Ratio, Jensen's Alpha
- **Stocks & Bonds**: Dividend Discount Model, Bond Price
- **Corporate**: WACC, Free Cash Flow to Firm, Sustainable Growth Rate

## Security

- `.env` files are excluded from version control. Never commit credentials.
- The `core` crate forbids `unsafe` code and uses `rust_decimal` for all financial arithmetic.
- The API enforces hard limits on graph size and simulation scale.

## Roadmap

See [`CASIROS_ROADMAP.md`](CASIROS_ROADMAP.md) for the full architectural blueprint, NASA/JPL coding standard adaptation, Clean Architecture layers, and the complete formula catalog.
