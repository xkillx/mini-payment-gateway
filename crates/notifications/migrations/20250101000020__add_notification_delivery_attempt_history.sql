-- Add retry_generation column
ALTER TABLE notification_delivery_records ADD COLUMN IF NOT EXISTS retry_generation INT NOT NULL DEFAULT 0;

-- Create attempt status enum
DO $$ BEGIN
    CREATE TYPE notification_delivery_attempt_status AS ENUM ('processing', 'delivered', 'failed');
EXCEPTION WHEN duplicate_object THEN NULL;
END $$;

-- Create attempt history table
CREATE TABLE IF NOT EXISTS notification_delivery_attempts (
    id UUID PRIMARY KEY,
    notification_delivery_record_id UUID NOT NULL REFERENCES notification_delivery_records(id),
    retry_generation INT NOT NULL,
    attempt_number INT NOT NULL,
    status notification_delivery_attempt_status NOT NULL,
    http_status_code INT,
    error TEXT,
    started_at TIMESTAMPTZ NOT NULL,
    finished_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT chk_retry_generation CHECK (retry_generation >= 0),
    CONSTRAINT chk_attempt_number CHECK (attempt_number > 0),
    CONSTRAINT chk_http_status_code CHECK (http_status_code IS NULL OR http_status_code BETWEEN 100 AND 599),
    CONSTRAINT chk_finished_consistent CHECK (
        (status = 'processing' AND finished_at IS NULL) OR
        (status IN ('delivered', 'failed') AND finished_at IS NOT NULL)
    ),
    CONSTRAINT chk_delivered_no_error CHECK (
        status != 'delivered' OR error IS NULL
    )
);

-- Unique per-record attempt numbers
CREATE UNIQUE INDEX IF NOT EXISTS idx_attempts_record_attempt
    ON notification_delivery_attempts (notification_delivery_record_id, attempt_number);

-- Index for generation-bounded queries
CREATE INDEX IF NOT EXISTS idx_attempts_record_generation_attempt
    ON notification_delivery_attempts (notification_delivery_record_id, retry_generation, attempt_number);

-- Partial index for active processing attempts
CREATE INDEX IF NOT EXISTS idx_attempts_processing
    ON notification_delivery_attempts (status) WHERE status = 'processing';
