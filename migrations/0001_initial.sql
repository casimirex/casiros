-- CASIROS snapshot persistence schema.

CREATE TABLE IF NOT EXISTS snapshots (
    id TEXT PRIMARY KEY,
    data JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
