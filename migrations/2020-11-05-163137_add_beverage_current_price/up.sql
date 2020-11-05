-- Your SQL goes here
ALTER TABLE beverages
ADD COLUMN current_price BIGINT NOT NULL DEFAULT 0;