CREATE TYPE notification_status AS ENUM ('pending', 'processing', 'delivered', 'failed');

CREATE TABLE IF NOT EXISTS notification_delivery_records (
    id UUID PRIMARY KEY,
    domain_event_id UUID NOT NULL REFERENCES domain_events(id),
    destination_url TEXT NOT NULL,
    status notification_status NOT NULL DEFAULT 'pending',
    attempt_count INT NOT NULL DEFAULT 0,
    last_attempt_at TIMESTAMPTZ,
    next_retry_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_notification_status ON notification_delivery_records (status);
CREATE INDEX idx_notification_next_retry ON notification_delivery_records (next_retry_at)
    WHERE status = 'pending';
CREATE INDEX idx_notification_event ON notification_delivery_records (domain_event_id);
