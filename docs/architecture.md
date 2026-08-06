# Architecture

## Clean Architecture Layers

CASIROS follows Clean Architecture principles with strict dependency rules:

```
Presentation:        web/ (dashboard), python/ (SDK)
                     ↓
Infrastructure:    casiros-api (HTTP server, Postgres, Redis, metrics)
                     ↓
Application:       casiros-dag (DAG engine, traits)
                   casiros-simulator (Monte Carlo runner)
                     ↓
Domain:            casiros-core (formulas, value objects, errors)
```

### Layer Rules

- **Domain** (`casiros-core`): Pure computations. No I/O, no web framework,
  no database. All monetary values use `rust_decimal::Decimal`.
- **Application** (`casiros-dag`, `casiros-simulator`): Orchestrates domain
  logic. Defines trait boundaries for persistence.
- **Infrastructure** (`casiros-api`): Implements traits defined in the
  application layer. Handles HTTP, WebSocket, database, and metrics.
- **Presentation** (`web/`, `python/`): User interfaces consuming the API.

## Domain Model

### Core Types

```
TenantId     — Globally unique tenant identifier
WorkspaceId  — Scoped workspace within a tenant
Principal    — Authenticated caller (tenant + workspace + key)
```

### Financial Formulas

55+ formulas organized by domain:

| Module | Formulas | Examples |
|---|---|---|
| `general` | 12 | FV, PV, NPV, IRR, annuity, perpetuity |
| `financial` | 14 | ROE, ROA, DuPont, Altman Z, ratios |
| `corporate` | 8 | WACC, FCFF, FCFE, EVA, tax shield |
| `markets` | 9 | Sharpe, Sortino, beta, VaR, SMA |
| `stocks_bonds` | 7 | DDM, DCF, duration, convexity |
| `options` | 9 | Black-Scholes, binomial, Greeks |
| `banking` | 4 | NIM, CAR, LDR, provision coverage |

### Audit Trail

```
AuditEvent {
    id: Uuid,
    timestamp: OffsetDateTime,
    principal: Principal,
    action: AuditAction,
    resource: String,
    result: AuditResult,
    metadata: HashMap<String, String>,
}
```

### Job Lifecycle

```
Queued → Running → Completed
                  → Failed
                  → Cancelled
```

## Data Flow

### Synchronous Evaluation

```
Client → POST /evaluate → auth_middleware → handler → EngineBuilder
  → CausalityEngine::evaluate() → Response
```

### Async Simulation

```
Client → POST /simulate/jobs → auth_middleware → handler
  → JobStore::enqueue() → 202 Accepted
                          ↓
Worker → JobStore::claim_next() → EngineBuilder
  → MonteCarloConfig::run_batch() → JobStore::update_progress()
  → JobStore::complete()
                          ↓
Client → GET /simulate/jobs/{id} → JobStore::get() → Response
```

## Storage

### PostgreSQL Tables

| Table | Purpose |
|---|---|
| `tenants` | Top-level organizational boundary |
| `workspaces` | Data partitions within a tenant |
| `api_keys` | Authentication credentials |
| `snapshots` | Persisted DAG snapshots |
| `audit_events` | Immutable audit trail |
| `simulation_jobs` | Async job queue |

### Redis

Optional cache for memoized formula evaluations. Configured via
`CASIROS__REDIS__URL` when the `redis` feature is enabled.

## Security

- All authenticated endpoints enforce tenant/workspace scoping.
- Admin endpoints require a separate `CASIROS_ADMIN_KEY`.
- Rate limiting is per tenant/workspace with optional per-key overrides.
- Audit trail is append-only with no update or delete operations.
- All crates forbid unsafe code (`#![forbid(unsafe_code)]`).
