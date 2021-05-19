-- Add up migration script here
CREATE TABLE orders (
    id BIGSERIAL PRIMARY KEY,
    game_id BIGSERIAL REFERENCES games(id),
    user_id BIGSERIAL REFERENCES users(id),
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT now()
);

ALTER TABLE transactions
ADD COLUMN order_id BIGINT DEFAULT NULL;

-- generate orders
INSERT INTO orders (id, game_id, user_id, created_at)
(SELECT nextval('orders_id_seq'::regclass), game_id, user_id, created_at FROM transactions);

-- make sure the order_id is NOT NULL
UPDATE transactions t
SET order_id = (
    SELECT id FROM orders o
    WHERE t.user_id=o.user_id AND t.game_id = o.game_id AND t.created_at = o.created_at
); 

ALTER TABLE transactions
ADD CONSTRAINT transactions_order_id_fkey
FOREIGN KEY (order_id)
REFERENCES orders(id);

ALTER TABLE transactions
ALTER COLUMN order_id SET NOT NULL;
