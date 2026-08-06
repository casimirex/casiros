# API Reference

The CASIROS API is documented via OpenAPI 3.1. The interactive Swagger UI
is available at `/swagger-ui` when the server is running.

## Exporting the Spec

```bash
cargo run -p casiros-api --bin casiros-api-export-openapi > casiros.openapi.json
```

## Endpoints

### Public Endpoints

These endpoints do not require authentication.

| Method | Path | Description |
|---|---|---|
| `GET` | `/healthz` | Liveness and readiness probe |
| `GET` | `/metrics` | Prometheus metrics in text format |
| `GET` | `/openapi.json` | OpenAPI 3.1 specification |
| `GET` | `/swagger-ui/*` | Interactive API documentation |

### Evaluation & Simulation

| Method | Path | Description |
|---|---|---|
| `POST` | `/evaluate` | Evaluate a DAG with fixed inputs |
| `POST` | `/schedule/amortization` | Generate a loan repayment schedule |
| `POST` | `/simulate` | Run a Monte Carlo simulation |
| `POST` | `/simulate/stream` | Streaming simulation (SSE) |
| `GET` | `/ws/simulate` | WebSocket simulation |

#### Why `/schedule/amortization` is not a formula

Every formula reachable through `/evaluate` returns a single `Decimal`, because
that is what a graph node evaluates to. An amortization schedule is a table —
one row per period — so it cannot be expressed that way without throwing away
the breakdown that makes it useful. It gets its own route instead.

`rate` is the rate **per period**, not per year. A 12% annual rate on a monthly
schedule is `0.01`. Requests are capped at 1,000 periods.

```bash
curl -X POST http://localhost:8080/schedule/amortization \
  -H "X-API-Key: $CASIROS_API_KEY" \
  -H 'Content-Type: application/json' \
  -d '{"principal": "1000.0", "rate": "0.01", "periods": 12}'
```

```json
{
  "payment": "88.84878867834170733998783123",
  "total_interest": "66.185464140100488079853974742",
  "schedule": [
    {
      "period": 1,
      "principal_paid": "78.84878867834170733998783123",
      "interest_paid": "10.000",
      "remaining_balance": "921.1512113216582926600121688"
    }
  ]
}
```

Values are decimal strings at full precision, never floats. Trailing scale
comes from the arithmetic rather than any formatting rule — the first period's
interest is exactly `10.000` here — so round for display rather than assuming a
fixed number of places.

`payment` is the level payment, identical every period. A request for zero
periods is legal and returns an empty schedule with a zero payment.

### Snapshots

| Method | Path | Description |
|---|---|---|
| `POST` | `/snapshots` | Save a DAG snapshot |
| `GET` | `/snapshots` | List stored snapshots |
| `GET` | `/snapshots/{id}` | Load a snapshot |
| `DELETE` | `/snapshots/{id}` | Delete a snapshot |

### Async Jobs

| Method | Path | Description |
|---|---|---|
| `POST` | `/simulate/jobs` | Enqueue a simulation job |
| `GET` | `/simulate/jobs/{id}` | Get job status and result |
| `POST` | `/simulate/jobs/{id}/cancel` | Cancel a queued or running job |
| `GET` | `/ws/jobs/{id}` | WebSocket job progress stream |

### Audit

| Method | Path | Description |
|---|---|---|
| `GET` | `/audit` | List audit events (tenant-scoped) |

### Admin (requires `X-Admin-Key` header)

| Method | Path | Description |
|---|---|---|
| `GET` | `/admin/tenants` | List tenants |
| `POST` | `/admin/tenants` | Provision a new tenant |
| `GET` | `/admin/tenants/{id}/stats` | Tenant usage statistics |
| `POST` | `/admin/keys` | Generate a new API key |
| `POST` | `/admin/keys/{id}/revoke` | Revoke an API key |

## Authentication

Protected endpoints require either:

- `Authorization: Bearer <key>` header, or
- `X-API-Key: <key>` header

Admin endpoints require the `X-Admin-Key` header set to the value of
`CASIROS_ADMIN_KEY`.

## Rate Limiting

Rate limiting is applied per tenant/workspace pair. The default limit is
60 requests per minute, configurable via `CASIROS_RATE_LIMIT_RPM` or
per-key via the 4th field in `CASIROS_API_KEY_TENANTS`.

## Error Responses

All endpoints return errors in a consistent format:

```json
{
  "error": "Human-readable error message"
}
```

HTTP status codes:

| Code | Meaning |
|---|---|
| 200 | Success |
| 202 | Accepted (async job enqueued) |
| 400 | Bad request (invalid input) |
| 401 | Unauthorized (missing or invalid API key) |
| 404 | Not found |
| 429 | Rate limit exceeded |
| 500 | Internal server error |
