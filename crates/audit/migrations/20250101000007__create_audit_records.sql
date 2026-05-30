CREATE TABLE IF NOT EXISTS audit_records (
    id UUID PRIMARY KEY,
    actor_id UUID NOT NULL REFERENCES actors(id),
    action TEXT NOT NULL,
    resource_type TEXT NOT NULL,
    resource_id TEXT NOT NULL,
    details JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_audit_actor ON audit_records (actor_id);
CREATE INDEX idx_audit_resource ON audit_records (resource_type, resource_id);
CREATE INDEX idx_audit_created_at ON audit_records (created_at);
