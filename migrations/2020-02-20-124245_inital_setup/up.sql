-- a game is an active rustfuif event, eg, people getting drunk
CREATE TABLE games (
    id BIGSERIAL PRIMARY KEY,
    name VARCHAR NOT NULL UNIQUE,
    start_time TIMESTAMP WITH TIME ZONE NOT NULL,
    duration_in_seconds INT DEFAULT 3600,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT now(),
    updated_at TIMESTAMP WITH TIME ZONE
);

-- a team is a group of users who want to play a game
-- a team configures their beverage prices/names/...
CREATE TABLE teams (
    id BIGSERIAL PRIMARY KEY,
    name VARCHAR NOT NULL,
    invitation UUID,
    game_id BIGSERIAL REFERENCES games(id),
    created_at TIMESTAMP WITH TIME ZONE DEFAULT now(),
    updated_at TIMESTAMP WITH TIME ZONE
);

-- slots are placeholders for beverages for a team
CREATE TABLE slots (
    id BIGSERIAL PRIMARY KEY,
    game_id BIGSERIAL REFERENCES games(id)
);
