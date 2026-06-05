ALTER TABLE reconciliations
  ADD COLUMN IF NOT EXISTS window_start TIMESTAMPTZ,
  ADD COLUMN IF NOT EXISTS window_end TIMESTAMPTZ,
  ADD COLUMN IF NOT EXISTS discrepancy_minor BIGINT,
  ADD COLUMN IF NOT EXISTS run_at TIMESTAMPTZ;

UPDATE reconciliations
SET window_start = created_at - INTERVAL '1 microsecond',
    window_end = created_at,
    discrepancy_minor = actual_total_minor - expected_total_minor,
    run_at = created_at
WHERE window_start IS NULL;

ALTER TABLE reconciliations
  ALTER COLUMN window_start SET NOT NULL,
  ALTER COLUMN window_end SET NOT NULL,
  ALTER COLUMN discrepancy_minor SET NOT NULL,
  ALTER COLUMN run_at SET NOT NULL;

ALTER TABLE reconciliations
  ADD CONSTRAINT chk_reconciliation_window CHECK (window_start < window_end);

CREATE INDEX IF NOT EXISTS idx_reconciliations_currency_window
  ON reconciliations (currency, window_start, window_end);
