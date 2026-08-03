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

Example for a Postgres-backed deployment:

```bash
export CASIROS_BIND_ADDR=0.0.0.0:8080
export CASIROS_SNAPSHOT__BACKEND=postgres
export CASIROS_POSTGRES__URL=postgresql://casiros:casiros@localhost:5432/casiros
export CASIROS_API__KEYS="prod-key-1,prod-key-2"
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

## Postgres Snapshots

When `CASIROS_SNAPSHOT__BACKEND=postgres`, the API automatically runs pending
SQLx migrations on startup. The migration file is at `migrations/0001_initial.sql`.

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
