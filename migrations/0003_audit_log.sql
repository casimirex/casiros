-- CASIROS immutable audit log schema.

CREATE TYPE audit_action AS ENUM (
    'evaluate',
    'simulate',
    'snapshot_create',
    'snapshot_read',
    'snapshot_delete',
    'job_create',
    'job_read',
    'job_cancel'
);

CREATE TYPE audit_result AS ENUM (
    'success',
    'forbidden',
    'not_found',
    'error'
);

CREATE TABLE IF NOT EXISTS audit_events (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id TEXT NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    workspace_id TEXT NOT NULL REFERENCES workspaces(id),
    api_key_id TEXT NOT NULL,
    action audit_action NOT NULL,
    resource TEXT NOT NULL,
    result audit_result NOT NULL,
    error_message TEXT,
    metadata JSONB NOT NULL DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_audit_events_tenant_workspace
    ON audit_events(tenant_id, workspace_id, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_audit_events_created_at
    ON audit_events(created_at DESC);
