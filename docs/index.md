# CASIROS

**NASA/JPL-grade Financial Physics Engine & Multiverse Simulator**

CASIROS is a financial computation platform that combines a deterministic
causality graph engine with a Monte Carlo multiverse simulator. It is built
to the NASA JPL Institutional Coding Standard for safety-critical systems.

## Key Features

- **44+ Financial Formulas** — Time value of money, corporate finance,
  options pricing (Black-Scholes, binomial), financial ratios, and more.
- **Causality DAG Engine** — Build directed acyclic graphs of financial
  formulas and evaluate them with automatic topological ordering.
- **Monte Carlo Simulator** — Run thousands of stochastic universes with
  configurable distributions and parallel execution.
- **REST API** — Full HTTP API with OpenAPI 3.1 specification, Swagger UI,
  and typed Rust and Python clients.
- **Multi-Tenant** — Tenant isolation, workspace scoping, and per-key
  rate limiting for production deployments.
- **Audit Trail** — Immutable, append-only audit logging for every
  authenticated request.
- **Async Jobs** — Long-running simulations are enqueued, executed by a
  background worker, and queried via REST or WebSocket.
- **Prometheus Metrics** — Request counts, durations, rate-limit events,
  and job state transitions.
- **Admin API** — Runtime tenant and API key management.

## Operator's Handbook

The [Operator's Handbook](handbook.html) is a complete 19-chapter manual taking
you from your first health check through multi-tenant production deployment.
Every command and response in it was captured from a running instance.

## Quick Start

```bash
# Start the API server
docker compose up -d

# Check health
curl http://localhost:8080/healthz

# Evaluate a simple DAG
curl -X POST http://localhost:8080/evaluate \
  -H "Content-Type: application/json" \
  -d '{"nodes":[{"input":{"name":"x"}},{"formula":{"name":"y","kind":{"formula":"future_value","present_value":{"node":"x"},"rate":0.05,"periods":10}}}],"edges":[{"dependency":"x","dependent":"y"}],"inputs":{"x":"100"}}'
```

## Project Status

| Version | Date | Highlights |
|---|---|---|
| v0.5.0 | 2026-08-05 | Python SDK, dashboard, formulas, CI/CD, security audit |
| v0.4.0 | 2026-08-05 | Metrics, rate-limit tiers, Redis cache, stateful worker, admin API |
| v0.3.0 | 2026-08-05 | Tenant isolation, audit trail, async jobs, background worker |
| v0.2.0 | 2026-08-03 | OpenAPI, API client, CLI, benchmarks, dashboard, Python SDK |
| v0.1.0 | 2026-08-01 | Initial MVP with core formulas and DAG engine |

## License

MIT OR Apache-2.0
