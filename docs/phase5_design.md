# Phase 5 Design — Production Control Plane

**Target release:** `v0.3.0`

**Theme:** Move CASIROS from a single-tenant API into a production-ready, multi-tenant, auditable, asynchronous simulation platform.

---

## 1. Objectives

- **Tenant / workspace isolation:** Every snapshot, simulation, and audit event belongs to exactly one `TenantId` + `WorkspaceId` pair.
- **Audit logging:** Immutable, append-only record of who did what to which resource, with result and timestamp.
- **Async simulation jobs:** Long-running Monte Carlo runs are enqueued, executed by a worker, and queried by clients.
- **Stateful simulation runner:** Partial aggregate checkpoints and best-effort cancel/resume.
- **DAG result cache:** Deterministic memoization for identical subgraph evaluations, with optional Redis backend.
- **Production release gates:** Keep the NASA/JPL-grade lint, test, and coverage bar while adding infrastructure.

---

## 2. Clean Architecture Placement

```text
Presentation:        web/, python/
Infrastructure:    casiros_api
  - tenant/auth middleware
  - PostgresTenantRepo, PostgresAuditLog, PostgresJobStore
  - job HTTP handlers + WebSocket pub/sub
  - casiros_worker binary
Application:       casiros_dag / casiros_simulator
  - SnapshotRepository (tenant-scoped)
  - AuditLog trait
  - JobQueue / JobStore traits
  - StatefulSimulationRunner
Domain:            casiros_core
  - TenantId, WorkspaceId, Principal
  - AuditEvent, JobId, JobStatus
  - Shared error variants
```

Rules:

- `casiros_core` remains pure: no I/O, no `actix-web`, no `sqlx`.
- All persistence is through traits defined in application crates and implemented in `casiros_api`.
- Every new crate uses `#![forbid(unsafe_code)]`, `#![deny(missing_docs)]`, `#![deny(clippy::pedantic)]`, `#![deny(warnings)]`.

---

## 3. Domain Model

### 3.1 Tenant and Workspace

```rust
/// Globally unique tenant identifier (e.g., `tenant_2vPShE...`).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TenantId(String);

/// Scoped workspace within a tenant (e.g., `workspace_...`).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct WorkspaceId(String);

/// Identity of the caller after authentication.
#[derive(Debug, Clone)]
pub struct Principal {
    pub tenant_id: TenantId,
    pub workspace_id: WorkspaceId,
    pub api_key_id: String,
}
```

### 3.2 Audit Event

```rust
#[derive(Debug, Clone)]
pub struct AuditEvent {
    pub id: Uuid,
    pub timestamp: OffsetDateTime,
    pub principal: Principal,
    pub action: AuditAction,
    pub resource: String,
    pub result: AuditResult,
    pub metadata: HashMap<String, String>,
}

pub enum AuditAction {
    Evaluate,
    Simulate,
    SnapshotCreate,
    SnapshotRead,
    SnapshotDelete,
    JobCreate,
    JobRead,
    JobCancel,
}

pub enum AuditResult {
    Success,
    Forbidden,
    NotFound,
    Error(String),
}
```

### 3.3 Simulation Job

```rust
#[derive(Debug, Clone)]
pub struct SimulationJob {
    pub id: JobId,
    pub tenant_id: TenantId,
    pub workspace_id: WorkspaceId,
    pub status: JobStatus,
    pub request: SimulateRequest,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
    pub progress: JobProgress,
    pub result: Option<SimulationResults>,
    pub error: Option<String>,
}

pub enum JobStatus {
    Queued,
    Running,
    Completed,
    Failed,
    Cancelled,
}

pub struct JobProgress {
    pub universes_total: usize,
    pub universes_completed: usize,
    pub last_checkpoint_at: Option<OffsetDateTime>,
}
```

---

## 4. Trait Boundaries

### 4.1 Tenant Resolution

```rust
#[async_trait]
pub trait TenantResolver: Send + Sync {
    async fn resolve(&self, api_key: &str) -> Option<Principal>;
}
```

### 4.2 Snapshot Repository (updated)

```rust
#[async_trait]
pub trait SnapshotRepository: Send + Sync {
    async fn save(
        &self,
        tenant: TenantId,
        workspace: WorkspaceId,
        snapshot: &EngineSnapshot,
    ) -> Result<SnapshotId, RepositoryError>;

    async fn load(
        &self,
        tenant: TenantId,
        workspace: WorkspaceId,
        id: &SnapshotId,
    ) -> Result<EngineSnapshot, RepositoryError>;

    async fn delete(
        &self,
        tenant: TenantId,
        workspace: WorkspaceId,
        id: &SnapshotId,
    ) -> Result<(), RepositoryError>;

    async fn list(
        &self,
        tenant: TenantId,
        workspace: WorkspaceId,
    ) -> Result<Vec<SnapshotSummary>, RepositoryError>;
}
```

### 4.3 Audit Log

```rust
#[async_trait]
pub trait AuditLog: Send + Sync {
    async fn record(&self, event: AuditEvent) -> Result<(), RepositoryError>;

    async fn list(
        &self,
        tenant: TenantId,
        pagination: Pagination,
    ) -> Result<Vec<AuditEvent>, RepositoryError>;
}
```

### 4.4 Job Store

```rust
#[async_trait]
pub trait JobStore: Send + Sync {
    async fn enqueue(&self, job: SimulationJob) -> Result<(), RepositoryError>;
    async fn claim_next(&self, worker_id: &str) -> Option<SimulationJob>;
    async fn update_progress(&self, id: &JobId, progress: JobProgress) -> Result<(), RepositoryError>;
    async fn complete(&self, id: &JobId, result: SimulationResults) -> Result<(), RepositoryError>;
    async fn fail(&self, id: &JobId, error: String) -> Result<(), RepositoryError>;
    async fn cancel(&self, id: &JobId) -> Result<bool, RepositoryError>;
    async fn get(&self, tenant: TenantId, workspace: WorkspaceId, id: &JobId)
        -> Result<SimulationJob, RepositoryError>;
}
```

### 4.5 Formula Cache

```rust
#[async_trait]
pub trait FormulaCache: Send + Sync {
    async fn get(&self, key: &CacheKey) -> Option<EvaluationResult>;
    async fn put(&self, key: CacheKey, value: EvaluationResult);
}
```

---

## 5. HTTP Endpoints

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/workspaces` | List workspaces for the tenant. |
| `POST` | `/simulate/jobs` | Enqueue a new simulation job. |
| `GET` | `/simulate/jobs/{id}` | Get job status and result (when completed). |
| `POST` | `/simulate/jobs/{id}/cancel` | Cancel a queued or running job. |
| `GET` | `/ws/jobs/{id}` | WebSocket stream of progress frames. |
| `GET` | `/audit` | Paginated audit log (tenant-scoped admin view). |

Public paths (`/healthz`, `/openapi.json`, `/swagger-ui/*`) remain unauthenticated.

---

## 6. Worker Binary

`casiros-worker` is a standalone binary in `crates/worker/`.

```text
loop:
  1. claim next Queued job from PostgresJobStore
  2. spawn StatefulSimulationRunner with cancellation token
  3. write progress checkpoint every N universes or seconds
  4. publish progress events to WebSocket pub/sub or SSE channel
  5. on completion: store SimulationResults in snapshots table, mark job Completed
  6. on failure: mark job Failed with error message
```

Workers can be scaled horizontally. Each worker registers a `worker_id` and uses advisory locks or `FOR UPDATE SKIP LOCKED` to avoid double claiming.

---

## 7. Database Schema

See migrations:

- `migrations/0002_tenants_and_workspaces.sql`
- `migrations/0003_audit_log.sql`
- `migrations/0004_simulation_jobs.sql`

Key tables:

- `tenants` — id, name, plan, created_at.
- `api_keys` — key hash, tenant_id, default_workspace_id, rate_limit_rpm, revoked_at.
- `workspaces` — id, tenant_id, name.
- `audit_events` — id, tenant_id, workspace_id, api_key_id, action, resource, result, metadata JSONB, timestamp.
- `simulation_jobs` — id, tenant_id, workspace_id, status, request JSONB, progress JSONB, result_snapshot_id, error, created_at, updated_at, claimed_by, claimed_until.

`snapshots` table will be migrated to add `tenant_id`, `workspace_id`, and `name` columns.

---

## 8. Implementation Slices

| Slice | Deliverable | Tests |
|-------|-------------|-------|
| 0 | Design doc + migrations | SQLx migration checksums valid |
| 1 | Core tenant/workspace model + updated SnapshotRepository trait | In-memory repo tests scoped by tenant |
| 2 | API-key → tenant mapping + tenant-scoped rate limiting | Key A cannot read key B snapshots |
| 3 | Audit log trait + Postgres impl + middleware | Every request leaves an event |
| 4 | Job model + in-memory JobStore + handlers | Enqueue/status/cancel lifecycle |
| 5 | Worker binary + Postgres JobStore + WebSocket progress | Worker completes a queued job |
| 6 | DAG result cache trait + in-memory + optional Redis | Benchmark before/after |
| 7 | Client updates, ops docs, CHANGELOG, v0.3.0 tag | All CI checks green |

---

## 9. Definition of Done

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo doc --workspace --no-deps --document-private-items
cargo tarpaulin --workspace --timeout 300 --fail-under 60
cargo audit
cargo deny check
```

Plus the standard CASIROS rules: no `.unwrap()` outside tests, every public item documented, ≥2 assertions per function, and OpenAPI spec regenerated.
