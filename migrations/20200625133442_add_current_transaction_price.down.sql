-- This file should undo anything in `up.sql`
ALTER TABLE transactions
DROP CONSTRAINT price_check;

ALTER TABLE transactions
DROP COLUMN price;

ALTER TABLE beverage_configs
ALTER COLUMN max_price TYPE INT;

ALTER TABLE beverage_configs
ALTER COLUMN min_price TYPE INT;

ALTER TABLE beverage_configs
ALTER COLUMN starting_price TYPE INT;