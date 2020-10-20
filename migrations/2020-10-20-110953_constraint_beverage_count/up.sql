-- Your SQL goes here
ALTER TABLE games
ADD CONSTRAINT game_at_least_two_beverages
CHECK (beverage_count > 1);