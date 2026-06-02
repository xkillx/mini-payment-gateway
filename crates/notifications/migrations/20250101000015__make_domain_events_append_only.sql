CREATE OR REPLACE FUNCTION reject_domain_event_modification()
RETURNS TRIGGER AS $$
BEGIN
    RAISE EXCEPTION 'domain_events are append-only; modification is not allowed'
        USING HINT = 'Insert a new domain event instead of modifying an existing one';
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trg_domain_events_append_only ON domain_events;

CREATE TRIGGER trg_domain_events_append_only
    BEFORE UPDATE OR DELETE ON domain_events
    FOR EACH ROW
    EXECUTE FUNCTION reject_domain_event_modification();
