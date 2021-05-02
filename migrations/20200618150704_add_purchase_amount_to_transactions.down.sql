-- This file should undo anything in `up.sql`
ALTER TABLE transactions
DROP CONSTRAINT purchase_amount_check;

ALTER TABLE transactions
DROP COLUMN amount;