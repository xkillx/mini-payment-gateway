CREATE UNIQUE INDEX IF NOT EXISTS idx_domain_events_payment_terminal_outcome_unique
    ON domain_events (aggregate_type, aggregate_id)
    WHERE aggregate_type = 'payment'
      AND event_type IN ('payment.successful', 'payment.failed');
