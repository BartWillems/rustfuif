-- Your SQL goes here
ALTER TABLE transactions
ADD COLUMN amount INT NOT NULL;

ALTER TABLE transactions
ADD CONSTRAINT purchase_amount_check 
CHECK (amount > 0);
