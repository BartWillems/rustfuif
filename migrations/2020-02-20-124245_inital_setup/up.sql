-- a game is an active rustfuif event, eg, people getting drunk
CREATE TABLE games (
    id BIGSERIAL PRIMARY KEY,
    name VARCHAR NOT NULL UNIQUE,
    start_time TIMESTAMP WITH TIME ZONE NOT NULL,
    close_time TIMESTAMP WITH TIME ZONE NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT now(),
    updated_at TIMESTAMP WITH TIME ZONE,
    CHECK(close_time > start_time)
);

-- participants are a group of users who want to play a game
-- participants configure their beverage prices/names/...
CREATE TABLE participants (
    id BIGSERIAL PRIMARY KEY,
    name VARCHAR NOT NULL,
    invitation UUID,
    game_id BIGSERIAL REFERENCES games(id),
    created_at TIMESTAMP WITH TIME ZONE DEFAULT now(),
    updated_at TIMESTAMP WITH TIME ZONE
);

-- slots are placeholders for beverages for participants
CREATE TABLE slots (
    id BIGSERIAL PRIMARY KEY,
    game_id BIGSERIAL REFERENCES games(id)
);

CREATE TABLE users (
    id BIGSERIAL PRIMARY KEY,
    username VARCHAR NOT NULL UNIQUE,
    password VARCHAR NOT NULL,
    is_admin boolean NOT NULL DEFAULT FALSE,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT now(),
    updated_at TIMESTAMP WITH TIME ZONE
);

-- automatically update `updated_at` columns
SELECT diesel_manage_updated_at('games');
SELECT diesel_manage_updated_at('participants');
SELECT diesel_manage_updated_at('users');
