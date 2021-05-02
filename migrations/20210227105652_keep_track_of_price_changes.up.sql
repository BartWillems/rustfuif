-- Your SQL goes here
CREATE TABLE price_histories (
    id BIGSERIAL PRIMARY KEY,
    game_id BIGSERIAL REFERENCES games(id),
    user_id BIGSERIAL REFERENCES users(id),
    slot_no SMALLINT NOT NULL,
    price BIGINT NOT NULL DEFAULT 0,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT now()
);
