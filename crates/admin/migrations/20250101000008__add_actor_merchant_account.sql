ALTER TABLE actors ADD COLUMN merchant_id UUID;

UPDATE actors SET merchant_id = id WHERE role = 'merchant';
UPDATE actors SET merchant_id = NULL WHERE role = 'administrator';

ALTER TABLE actors ALTER COLUMN merchant_id DROP NOT NULL;

ALTER TABLE actors ADD CONSTRAINT chk_actor_merchant_id_role
    CHECK (
        (role = 'merchant' AND merchant_id IS NOT NULL)
        OR (role = 'administrator' AND merchant_id IS NULL)
    );
