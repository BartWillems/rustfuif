-- Add down migration script here
ALTER TABLE transactions
ADD COLUMN user_id BIGINT DEFAULT NULL,
ADD COLUMN game_id BIGINT DEFAULT NULL,
ADD COLUMN created_at TIMESTAMP WITH TIME ZONE DEFAULT NULL;

-- Restore user_id
UPDATE transactions t
SET user_id = (
    SELECT user_id FROM orders o
    WHERE t.order_id=o.id
);

ALTER TABLE transactions
ADD CONSTRAINT transactions_user_id_fkey
FOREIGN KEY (user_id)
REFERENCES users(id);

ALTER TABLE transactions
ALTER COLUMN user_id SET NOT NULL;

-- Restore game_id
UPDATE transactions t
SET game_id = (
    SELECT game_id FROM orders o
    WHERE t.order_id=o.id
);

ALTER TABLE transactions
ADD CONSTRAINT transactions_game_id_fkey
FOREIGN KEY (game_id)
REFERENCES games(id);

ALTER TABLE transactions
ALTER COLUMN game_id SET NOT NULL;

-- Restore created_at
UPDATE transactions t
SET created_at = (
    SELECT created_at FROM orders o
    WHERE t.order_id=o.id
);

ALTER TABLE transactions
ALTER COLUMN created_at SET NOT NULL;
