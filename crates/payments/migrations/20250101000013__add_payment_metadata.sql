ALTER TABLE payments ADD COLUMN IF NOT EXISTS metadata JSONB NOT NULL DEFAULT '{}'::jsonb;

ALTER TABLE payments ADD CONSTRAINT payments_metadata_is_object CHECK (jsonb_typeof(metadata) = 'object');
