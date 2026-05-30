ALTER TABLE audit_records ADD COLUMN actor_type TEXT;

UPDATE audit_records SET actor_type = COALESCE(
    (SELECT a.role FROM actors a WHERE a.id = audit_records.actor_id),
    'unknown'
);

ALTER TABLE audit_records ALTER COLUMN actor_type SET NOT NULL;

ALTER TABLE audit_records ADD CONSTRAINT chk_audit_actor_type
    CHECK (actor_type IN ('merchant', 'administrator', 'system', 'unknown'));

ALTER TABLE audit_records DROP CONSTRAINT IF EXISTS audit_records_actor_id_fkey;

ALTER TABLE audit_records ALTER COLUMN actor_id DROP NOT NULL;
