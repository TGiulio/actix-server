-- Add migration script here
-- we make this transaction to uniform old subscriber that may have status NULL
BEGIN;
    UPDATE subscriptions
        SET status = 'confirmed'
        WHERE status IS NULL;
    -- actually make status mandatory
    ALTER TABLE subscriptions ALTER COLUMN status SET NOT NULL;
COMMIT;