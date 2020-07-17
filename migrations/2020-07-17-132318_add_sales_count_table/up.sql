-- Your SQL goes here
CREATE TABLE sales_counts (
    game_id BIGSERIAL REFERENCES games(id),
    slot_no SMALLINT NOT NULL,
    sales BIGINT NOT NULL DEFAULT 0,
    CONSTRAINT sales_count_pkey PRIMARY KEY (game_id, slot_no),
    CHECK (sales >= 0)
);