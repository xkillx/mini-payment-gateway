ALTER TABLE payments ADD COLUMN IF NOT EXISTS failure_reason TEXT;

ALTER TABLE payments
    DROP CONSTRAINT IF EXISTS payments_failure_reason_only_when_failed;
ALTER TABLE payments
    ADD CONSTRAINT payments_failure_reason_only_when_failed
    CHECK (status = 'failed' OR failure_reason IS NULL);

CREATE UNIQUE INDEX IF NOT EXISTS idx_domain_events_payment_lifecycle_unique
    ON domain_events (aggregate_type, aggregate_id, event_type)
    WHERE aggregate_type = 'payment'
      AND event_type IN (
        'payment.created',
        'payment.processing',
        'payment.successful',
        'payment.failed',
        'payment.refunded'
      );
