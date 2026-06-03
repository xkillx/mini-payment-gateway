DROP INDEX IF EXISTS idx_refunds_payment_id;
CREATE UNIQUE INDEX IF NOT EXISTS idx_refunds_payment_id_unique ON refunds (payment_id);
