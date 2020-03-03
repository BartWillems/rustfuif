CREATE TABLE users (
    id BIGSERIAL PRIMARY KEY,
    username VARCHAR NOT NULL UNIQUE,
    password VARCHAR NOT NULL,
    is_admin BOOLEAN NOT NULL DEFAULT FALSE,
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

-- invitations are used to add users to a game
CREATE TABLE invitations (
    user_id BIGSERIAL REFERENCES users(id),
    game_id BIGSERIAL REFERENCES games(id),
    state VARCHAR NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT now(),
    updated_at TIMESTAMP WITH TIME ZONE,
    CONSTRAINT user_game_pkey PRIMARY KEY (user_id, game_id),
    CHECK(state = 'PENDING' OR state = 'ACCEPTED' OR state = 'DECLINED')
);

-- a transaction is a sale
CREATE TABLE transactions (
    id BIGSERIAL PRIMARY KEY,
    user_id BIGSERIAL REFERENCES users(id),
    game_id BIGSERIAL REFERENCES games(id),
    slot_no SMALLINT NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT now(),
    -- to keep it simple, a game has hardcoded 8 slots
    -- this could be improved by adding a slot_limit to a game and checking against that limit
    CHECK (slot_no > 0 AND slot_no <= 8)
);

-- automatically update `updated_at` columns
SELECT diesel_manage_updated_at('games');
SELECT diesel_manage_updated_at('users');
SELECT diesel_manage_updated_at('invitations');

-- create initial admin:admin account
INSERT INTO users (username, password, is_admin)
VALUES ('admin', '$argon2i$v=19$m=4096,t=3,p=1$KA0uyctXkrYJu6+EdkwMcecm97DkFJL1yvFOumns9AM$7XEYsYEd40Z1V0o8mDCoLldu7VTxXA20hXyl/x28LlM', TRUE);