ALTER TABLE audit_records ADD COLUMN occurred_at TIMESTAMPTZ;

UPDATE audit_records SET occurred_at = created_at;

ALTER TABLE audit_records ALTER COLUMN occurred_at SET NOT NULL;

CREATE INDEX idx_audit_timeline ON audit_records (occurred_at DESC, created_at DESC, id DESC);

CREATE OR REPLACE FUNCTION reject_audit_record_modification()
RETURNS TRIGGER AS $$
BEGIN
    RAISE EXCEPTION 'audit_records are append-only; modification is not allowed'
        USING HINT = 'Insert a new audit record instead of modifying an existing one';
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trg_audit_append_only
    BEFORE UPDATE OR DELETE ON audit_records
    FOR EACH ROW
    EXECUTE FUNCTION reject_audit_record_modification();
