-- Your SQL goes here
ALTER TABLE games
ADD COLUMN beverage_count SMALLINT NOT NULL DEFAULT 8;

ALTER TABLE transactions
DROP CONSTRAINT transactions_slot_no_check;

-- the hardcoded max is removed
ALTER TABLE transactions
ADD CONSTRAINT transactions_slot_no_check
CHECK (slot_no >= 0);

ALTER TABLE beverage_configs
DROP CONSTRAINT beverage_configs_slot_no_check;

ALTER TABLE beverage_configs
ADD CONSTRAINT beverage_configs_slot_no_check
CHECK (slot_no >= 0);