CREATE TABLE IF NOT EXISTS idempotency_records (
    id UUID PRIMARY KEY,
    merchant_id UUID NOT NULL REFERENCES actors(id),
    command_namespace TEXT NOT NULL,
    idempotency_key TEXT NOT NULL,
    response_code TEXT NOT NULL,
    response_body JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE UNIQUE INDEX idx_idempotency_unique ON idempotency_records (merchant_id, command_namespace, idempotency_key);
