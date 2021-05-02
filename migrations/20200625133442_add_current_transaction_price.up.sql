-- Your SQL goes here-- Your SQL goes here
ALTER TABLE transactions
ADD COLUMN price BIGINT NOT NULL DEFAULT 0;

ALTER TABLE transactions
ADD CONSTRAINT price_check
CHECK (price >= 0);

ALTER TABLE beverage_configs
ALTER COLUMN max_price TYPE BIGINT;

ALTER TABLE beverage_configs
ALTER COLUMN min_price TYPE BIGINT;

ALTER TABLE beverage_configs
ALTER COLUMN starting_price TYPE BIGINT;