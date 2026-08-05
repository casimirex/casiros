# CASIROS Operator Runbook

This guide covers how to configure, deploy, observe, and maintain a CASIROS
API server in production-like environments.

## Table of Contents

1. [Configuration](#configuration)
2. [Deployment](#deployment)
3. [Postgres Snapshots](#postgres-snapshots)
4. [Health Checks](#health-checks)
5. [API Key Rotation](#api-key-rotation)
6. [Observability](#observability)
7. [Upgrading](#upgrading)
8. [Troubleshooting](#troubleshooting)

## Configuration

CASIROS uses layered configuration (lowest to highest precedence):

1. Embedded defaults in `config/default.toml`.
2. Optional `config/default.toml` found at the current working directory.
3. Environment variables prefixed with `CASIROS_`, using `__` as a nested-key
   separator.

### Common Settings

| Environment Variable | Default | Description |
|---|---|---|
| `CASIROS_BIND_ADDR` | `127.0.0.1:8080` | HTTP/WS bind address. Use `0.0.0.0:8080` in containers. |
| `CASIROS_LOG_LEVEL` | `info` | Log level (`trace`, `debug`, `info`, `warn`, `error`). |
| `CASIROS_RATE_LIMIT_RPM` | `60` | Per-key rate limit in requests per minute. |
| `CASIROS_SNAPSHOT__BACKEND` | `memory` | `memory` or `postgres`. |
| `CASIROS_POSTGRES__URL` | — | Postgres connection string when backend is `postgres`. |
| `CASIROS_API__KEYS` | — | Comma-separated API keys for authenticated endpoints. |
| `CASIROS_API_KEY_TENANTS` | — | Key-to-tenant mapping: `key1:tenant_1:workspace_1,key2:tenant_2:workspace_2`. When unset, all keys share the default tenant. |
| `CASIROS_RATE_LIMIT_RPM` | `60` | Per-tenant/workspace rate limit in requests per minute. |

Example for a Postgres-backed deployment with tenant isolation:

```bash
export CASIROS_BIND_ADDR=0.0.0.0:8080
export CASIROS_SNAPSHOT__BACKEND=postgres
export CASIROS_POSTGRES__URL=postgresql://casiros:casiros@localhost:5432/casiros
export CASIROS_API__KEYS="prod-key-1,prod-key-2"
export CASIROS_API_KEY_TENANTS="prod-key-1:tenant_acme:workspace_prod,prod-key-2:tenant_beta:workspace_staging"
cargo run -p casiros-api
```

## Deployment

### Docker Compose (recommended for single-node)

```bash
docker compose up -d --build
```

The bundled `docker-compose.yml` starts:

- `casiros-api` on port `8080`.
- `postgres:16-alpine` on port `5432` with a persistent named volume.
- API healthcheck via `GET /healthz`.
- Postgres healthcheck via `pg_isready`.

### Kubernetes / Helm Checklist

- Mount `config/default.toml` as a `ConfigMap` if you want to override defaults.
- Store `CASIROS_API__KEYS` and `CASIROS_POSTGRES__URL` in a `Secret`.
- Expose port `8080` for HTTP and WebSocket traffic.
- Use `GET /healthz` for both liveness and readiness probes.
- Run SQLx migrations before starting new pods (see below).

## Tenant Isolation

When `CASIROS_API_KEY_TENANTS` is set, each API key maps to a specific tenant
and workspace. All snapshots, audit events, and simulation jobs are scoped to
the caller's tenant. Cross-tenant access is rejected at the storage layer.

The default tenant/workspace (`tenant_default`/`workspace_default`) is used
when no mapping is configured. This preserves backward compatibility for
single-tenant deployments.

## Audit Log

Every authenticated request leaves an immutable audit event recording who did
what, to which resource, and how it turned out. The trail is append-only: there
is no update or delete operation.

- `GET /audit` returns the calling tenant's events, newest first.
- Query parameters: `limit` (clamped to 1–1000) and `offset`.
- Audit writes are best-effort: a backend failure is logged but does not fail
  the request. Monitor the `audit.write_failed` log target for alerts.

### Audit Events Table

| Column | Type | Description |
|---|---|---|
| `id` | UUID | Unique event identifier |
| `tenant_id` | TEXT | Owning tenant |
| `workspace_id` | TEXT | Owning workspace |
| `api_key_id` | TEXT | API key used |
| `action` | ENUM | `evaluate`, `simulate`, `snapshot_create`, `snapshot_read`, `snapshot_delete`, `job_create`, `job_read`, `job_cancel` |
| `resource` | TEXT | Path or resource identifier |
| `result` | ENUM | `success`, `forbidden`, `not_found`, `error` |
| `metadata` | JSONB | HTTP method, status code, and similar |

## Async Simulation Jobs

Long-running simulations can be enqueued as background jobs and queried
asynchronously:

- `POST /simulate/jobs` — enqueue a new simulation job (returns 202 with a
  job ID).
- `GET /simulate/jobs/{id}` — poll job status, progress, and result.
- `POST /simulate/jobs/{id}/cancel` — cancel a queued or running job.
- `GET /ws/jobs/{id}` — WebSocket that streams progress frames every 500ms
  until the job completes or fails.

### Job Lifecycle

```
Queued → Running → Completed
                  → Failed
                  → Cancelled
```

### Worker Binary

The `casiros-worker` binary claims and executes queued jobs:

```bash
export CASIROS_POSTGRES__URL=postgresql://casiros:casiros@localhost:5432/casiros
cargo run -p casiros-worker
```

Multiple workers can run concurrently. Each uses `FOR UPDATE SKIP LOCKED` to
avoid double-claiming. Workers poll every 5 seconds when the queue is empty.

## Postgres Snapshots

When `CASIROS_SNAPSHOT__BACKEND=postgres`, the API automatically runs pending
SQLx migrations on startup. The migration files are at `migrations/0001_initial.sql`
through `migrations/0004_simulation_jobs.sql`.

### Manual Migration

If you prefer to run migrations out-of-band (e.g. in CI or an init container):

```bash
cargo run -p casiros-api --bin casiros-api-migrate
```

> This binary is available if the API crate defines a migration entry point;
> otherwise use `sqlx migrate run` against `migrations/`.

### Backup / Restore

Use standard Postgres tooling:

```bash
# Backup
pg_dump -Fc -h localhost -U casiros casiros > casiros.snapshot.dump

# Restore
pg_restore -h localhost -U casiros -d casiros casiros.snapshot.dump
```

## Health Checks

```bash
curl -fsS http://localhost:8080/healthz
```

Expected response:

```json
{"status":"ok"}
```

The `healthz` endpoint is public and does not require an API key.

## API Key Rotation

API keys are provided at startup via `CASIROS_API__KEYS` as a comma-separated
list. To rotate:

1. Add the new key to `CASIROS_API__KEYS` alongside the old key.
2. Restart/redeploy all API instances.
3. Update clients to use the new key.
4. Remove the old key and restart/redeploy again.

There is no runtime key management API in the current release.

## Observability

### Logging

CASIROS emits structured `tracing` logs. Each HTTP request receives a span
containing the method, path, status code, and latency. Configure the log level
with `CASIROS_LOG_LEVEL`.

Example output:

```text
2026-08-03T14:00:00.000000Z  INFO request: method=POST path=/simulate status=200 duration_ms=12
```

### Metrics (Phase 5)

No built-in metrics exporter is present yet. The next phase will add
Prometheus-compatible `/metrics` covering request latency, rate-limit events,
and simulation throughput.

### Tracing

The custom `TracingMiddleware` creates a `tracing` span per request. It records:

- `http.method`
- `http.path`
- `http.status_code`
- `duration_ms`

Future phases will add OpenTelemetry export.

## Upgrading

1. Review `CHANGELOG.md` (or git log) for breaking changes.
2. If Postgres schema changed, run migrations before starting the new version.
3. Verify the new container with `GET /healthz`.
4. Confirm key endpoints with sample `POST /evaluate` and `POST /simulate` calls.

## Troubleshooting

### Service starts but snapshots fail

- Check `CASIROS_SNAPSHOT__BACKEND` is set to the intended value.
- For Postgres, verify `CASIROS_POSTGRES__URL` and that migrations have run.
- Look for `DagError::Repository` in logs.

### High latency on `/simulate`

- Increase `universe_count` gradually and profile with `crates/bench`.
- Consider running simulations on a dedicated worker pool in Phase 5.

### WebSocket disconnects

- The `/ws/simulate` endpoint expects a single JSON text frame and then streams
  progress/result/error frames. Ensure the client sends one complete text frame
  and handles ping/pong.

### 401 / 429 Errors

- 401: `Authorization: Bearer <key>` header is missing or invalid.
- 429: The configured rate limit was exceeded; see `CASIROS_RATE_LIMIT_RPM`.
  Per-key rate limits can be set via the 4th field in `CASIROS_API_KEY_TENANTS`.

## Metrics

CASIROS exposes a Prometheus-compatible `/metrics` endpoint at `GET /metrics`.
This endpoint is public and does not require authentication.

### Available Metrics

| Metric | Type | Labels | Description |
|---|---|---|---|
| `casiros_http_requests_total` | Counter | method, path, status | Total HTTP requests |
| `casiros_http_request_duration_seconds` | Histogram | method, path | Request duration |
| `casiros_rate_limit_denials_total` | Counter | tenant | Rate-limit denials |
| `casiros_jobs_total` | Counter | status | Job state transitions |
| `casiros_audit_write_failures_total` | Counter | — | Audit write failures |

### Prometheus Scrape Configuration

```yaml
scrape_configs:
  - job_name: 'casiros'
    scrape_interval: 15s
    metrics_path: /metrics
    static_configs:
      - targets: ['localhost:8080']
```

## Admin API

The admin API provides runtime management of tenants and API keys. All admin
endpoints require the `X-Admin-Key` header set to the value of
`CASIROS_ADMIN_KEY`.

### Endpoints

| Method | Path | Description |
|---|---|---|
| `GET` | `/admin/tenants` | List all tenants |
| `POST` | `/admin/tenants` | Provision a new tenant |
| `GET` | `/admin/tenants/{id}/stats` | Usage statistics for a tenant |
| `POST` | `/admin/keys` | Generate a new API key |
| `POST` | `/admin/keys/{id}/revoke` | Revoke an API key |

### Example

```bash
export CASIROS_ADMIN_KEY="admin-secret-123"

# List tenants
curl -H "X-Admin-Key: admin-secret-123" http://localhost:8080/admin/tenants

# Create a new API key
curl -X POST -H "X-Admin-Key: admin-secret-123" \
  -H "Content-Type: application/json" \
  -d '{"tenant_id":"tenant_acme","workspace_id":"workspace_prod"}' \
  http://localhost:8080/admin/keys
```

## Redis Cache

When the `redis` feature is enabled, the DAG formula cache can use Redis
instead of in-memory storage. Configure via environment variables:

| Variable | Default | Description |
|---|---|---|
| `CASIROS_REDIS__URL` | `redis://127.0.0.1:6379` | Redis connection URL |
| `CASIROS_REDIS__TTL` | `3600` | Cache entry TTL in seconds |
