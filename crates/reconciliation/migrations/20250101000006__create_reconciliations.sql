CREATE TYPE reconciliation_status AS ENUM ('matched', 'mismatched', 'error');

CREATE TABLE IF NOT EXISTS reconciliations (
    id UUID PRIMARY KEY,
    status reconciliation_status NOT NULL DEFAULT 'matched',
    expected_total_minor BIGINT NOT NULL,
    actual_total_minor BIGINT NOT NULL,
    currency TEXT NOT NULL,
    notes TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_reconciliations_status ON reconciliations (status);
CREATE INDEX idx_reconciliations_created_at ON reconciliations (created_at);
