-- CASIROS tenant/workspace isolation schema.

CREATE TABLE IF NOT EXISTS tenants (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    plan TEXT NOT NULL DEFAULT 'standard',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS workspaces (
    id TEXT PRIMARY KEY,
    tenant_id TEXT NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (tenant_id, name)
);

CREATE TABLE IF NOT EXISTS api_keys (
    id TEXT PRIMARY KEY,
    tenant_id TEXT NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    default_workspace_id TEXT NOT NULL REFERENCES workspaces(id),
    key_hash TEXT NOT NULL UNIQUE,
    name TEXT NOT NULL,
    rate_limit_rpm INTEGER NOT NULL DEFAULT 100,
    revoked_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Seed the default tenant/workspace referenced by the column defaults below and
-- by the API's fallback principal. Without these rows every insert into
-- snapshots would violate the foreign keys added at the end of this migration.
INSERT INTO tenants (id, name, plan)
    VALUES ('tenant_default', 'Default Tenant', 'standard')
    ON CONFLICT (id) DO NOTHING;

INSERT INTO workspaces (id, tenant_id, name)
    VALUES ('workspace_default', 'tenant_default', 'default')
    ON CONFLICT (id) DO NOTHING;

ALTER TABLE snapshots
    ADD COLUMN IF NOT EXISTS tenant_id TEXT NOT NULL DEFAULT 'tenant_default',
    ADD COLUMN IF NOT EXISTS workspace_id TEXT NOT NULL DEFAULT 'workspace_default',
    ADD COLUMN IF NOT EXISTS name TEXT;

ALTER TABLE snapshots
    ADD CONSTRAINT fk_snapshots_tenant FOREIGN KEY (tenant_id) REFERENCES tenants(id) ON DELETE CASCADE,
    ADD CONSTRAINT fk_snapshots_workspace FOREIGN KEY (workspace_id) REFERENCES workspaces(id) ON DELETE CASCADE;

CREATE INDEX IF NOT EXISTS idx_snapshots_tenant_workspace ON snapshots(tenant_id, workspace_id);
