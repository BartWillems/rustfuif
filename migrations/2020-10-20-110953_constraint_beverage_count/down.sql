-- This file should undo anything in `up.sql`
ALTER TABLE games
DROP CONSTRAINT game_at_least_two_beverages;