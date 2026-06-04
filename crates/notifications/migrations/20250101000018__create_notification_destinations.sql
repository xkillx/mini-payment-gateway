CREATE TABLE IF NOT EXISTS notification_destinations (
    id UUID PRIMARY KEY,
    merchant_id UUID NOT NULL,
    destination_url TEXT NOT NULL,
    is_active BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_notification_destinations_one_active_per_merchant
    ON notification_destinations (merchant_id) WHERE is_active = true;

CREATE INDEX IF NOT EXISTS idx_notification_destinations_merchant_active
    ON notification_destinations (merchant_id) WHERE is_active = true;

CREATE UNIQUE INDEX IF NOT EXISTS idx_notification_delivery_records_event_destination_unique
    ON notification_delivery_records (domain_event_id, destination_url);
