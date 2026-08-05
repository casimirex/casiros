-- CASIROS async simulation job queue schema.

CREATE TYPE job_status AS ENUM (
    'queued',
    'running',
    'completed',
    'failed',
    'cancelled'
);

CREATE TABLE IF NOT EXISTS simulation_jobs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id TEXT NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    workspace_id TEXT NOT NULL REFERENCES workspaces(id),
    status job_status NOT NULL DEFAULT 'queued',
    request JSONB NOT NULL,
    progress JSONB NOT NULL DEFAULT '{"universes_total":0,"universes_completed":0,"last_checkpoint_at":null}',
    result_snapshot_id TEXT REFERENCES snapshots(id),
    error_message TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    claimed_by TEXT,
    claimed_until TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS idx_simulation_jobs_status_claimed
    ON simulation_jobs(status, claimed_until)
    WHERE status = 'queued' OR status = 'running';

CREATE INDEX IF NOT EXISTS idx_simulation_jobs_tenant_workspace
    ON simulation_jobs(tenant_id, workspace_id, created_at DESC);

CREATE OR REPLACE FUNCTION update_simulation_jobs_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = now();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trg_simulation_jobs_updated_at ON simulation_jobs;
CREATE TRIGGER trg_simulation_jobs_updated_at
    BEFORE UPDATE ON simulation_jobs
    FOR EACH ROW
    EXECUTE FUNCTION update_simulation_jobs_updated_at();
