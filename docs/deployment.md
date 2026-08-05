# Deployment

## Configuration

CASIROS uses layered configuration (lowest to highest precedence):

1. Embedded defaults in `config/default.toml`.
2. Optional `config/default.toml` at the current working directory.
3. Environment variables prefixed with `CASIROS_`, using `__` as a
   nested-key separator.

### Common Settings

| Variable | Default | Description |
|---|---|---|
| `CASIROS__BIND_ADDR` | `127.0.0.1:8080` | HTTP/WS bind address |
| `CASIROS__LOG_LEVEL` | `info` | Log level |
| `CASIROS_RATE_LIMIT_RPM` | `60` | Per-tenant rate limit |
| `CASIROS_API_KEYS` | — | Comma-separated API keys |
| `CASIROS_API_KEY_TENANTS` | — | Key-to-tenant mapping |
| `CASIROS_ADMIN_KEY` | — | Admin API key |
| `CASIROS__SNAPSHOT__BACKEND` | `memory` | `memory` or `postgres` |
| `CASIROS__POSTGRES__URL` | — | Postgres connection string |
| `CASIROS_OTLP_ENDPOINT` | — | OpenTelemetry collector URL |

### Tenant Mapping

```text
CASIROS_API_KEY_TENANTS="key1:tenant_a:workspace_prod,key2:tenant_b:workspace_staging:200"
```

Format: `key:tenant_id:workspace_id` with an optional 4th field for
per-key rate limit in requests per minute.

## Docker Compose

```bash
docker compose up -d --build
```

This starts:
- `casiros-api` on port 8080
- `casiros-worker` (background job processor)
- `postgres:16-alpine` on port 5432
- `redis:7-alpine` on port 6379

## Docker

```bash
# Build the API image
docker build -t casiros-api --target api .

# Build the worker image
docker build -t casiros-worker --target worker .

# Run the API
docker run -p 8080:8080 casiros-api
```

## Kubernetes

### Prerequisites

- A PostgreSQL instance (managed or self-hosted)
- A Redis instance (optional, for cache)

### Environment Variables

Create a Secret:

```yaml
apiVersion: v1
kind: Secret
metadata:
  name: casiros-config
stringData:
  CASIROS_API_KEYS: "prod-key-1,prod-key-2"
  CASIROS__POSTGRES__URL: "postgresql://user:pass@host:5432/casiros"
  CASIROS_ADMIN_KEY: "admin-secret"
```

### Deployment

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: casiros-api
spec:
  replicas: 3
  selector:
    matchLabels:
      app: casiros-api
  template:
    metadata:
      labels:
        app: casiros-api
    spec:
      containers:
      - name: api
        image: casiros-api:latest
        ports:
        - containerPort: 8080
        envFrom:
        - secretRef:
            name: casiros-config
        livenessProbe:
          httpGet:
            path: /healthz
            port: 8080
        readinessProbe:
          httpGet:
            path: /healthz
            port: 8080
```

## Database Migrations

Migrations run automatically on startup when using the Postgres backend.
The migration files are in `migrations/`:

| File | Description |
|---|---|
| `0001_initial.sql` | Initial snapshots table |
| `0002_tenants_and_workspaces.sql` | Tenant/workspace isolation |
| `0003_audit_log.sql` | Audit events table |
| `0004_simulation_jobs.sql` | Async job queue |

## Monitoring

### Health Checks

```bash
curl -fsS http://localhost:8080/healthz
```

### Metrics

```bash
curl http://localhost:8080/metrics
```

Available metrics:

| Metric | Type | Description |
|---|---|---|
| `casiros_http_requests_total` | Counter | Request count by method, path, status |
| `casiros_http_request_duration_seconds` | Histogram | Request duration |
| `casiros_rate_limit_denials_total` | Counter | Rate-limit denials by tenant |
| `casiros_jobs_total` | Counter | Job state transitions |
| `casiros_audit_write_failures_total` | Counter | Audit write failures |

### OpenTelemetry

Set `CASIROS_OTLP_ENDPOINT` to enable OpenTelemetry export via
HTTP/protobuf. Requires the `otel` feature flag at build time.
