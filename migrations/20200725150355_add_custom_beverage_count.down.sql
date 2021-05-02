-- This file should undo anything in `up.sql`
ALTER TABLE games
DROP COLUMN beverage_count;

ALTER TABLE transactions
ADD CONSTRAINT transactions_slot_no_check
CHECK (slot_no >= 0 AND slot_no < 8);

ALTER TABLE beverage_configs
ADD CONSTRAINT beverage_configs_slot_no_check
CHECK (slot_no >= 0 AND slot_no < 8);
