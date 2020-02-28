CREATE TABLE users (
    id BIGSERIAL PRIMARY KEY,
    username VARCHAR NOT NULL UNIQUE,
    password VARCHAR NOT NULL,
    is_admin boolean NOT NULL DEFAULT FALSE,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT now(),
    updated_at TIMESTAMP WITH TIME ZONE
);

-- a game is an active rustfuif event, eg, people getting drunk
CREATE TABLE games (
    id BIGSERIAL PRIMARY KEY,
    name VARCHAR NOT NULL UNIQUE,
    owner_id BIGSERIAL REFERENCES users(id),
    start_time TIMESTAMP WITH TIME ZONE NOT NULL,
    close_time TIMESTAMP WITH TIME ZONE NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT now(),
    updated_at TIMESTAMP WITH TIME ZONE,
    CHECK(close_time > start_time)
);

CREATE TABLE users_games (
    user_id BIGSERIAL REFERENCES users(id),
    game_id BIGSERIAL REFERENCES games(id),
    CONSTRAINT user_game_pkey PRIMARY KEY (user_id, game_id)
);

-- slots are placeholders for beverages for participants
CREATE TABLE slots (
    id BIGSERIAL PRIMARY KEY,
    game_id BIGSERIAL REFERENCES games(id)
);

-- automatically update `updated_at` columns
SELECT diesel_manage_updated_at('games');
SELECT diesel_manage_updated_at('users');
