-- Add down migration script here
ALTER TABLE transactions
DROP COLUMN IF EXISTS order_id; 

DROP TABLE IF EXISTS orders;

