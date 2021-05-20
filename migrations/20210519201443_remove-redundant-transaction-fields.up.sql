-- Add up migration script here

ALTER TABLE transactions
DROP COLUMN user_id,
DROP COLUMN game_id,
DROP COLUMN created_at;
