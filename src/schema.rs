table! {
    games (id) {
        id -> Int8,
        name -> Varchar,
        start_time -> Timestamptz,
        close_time -> Timestamptz,
        created_at -> Nullable<Timestamptz>,
        updated_at -> Nullable<Timestamptz>,
    }
}

table! {
    participants (id) {
        id -> Int8,
        name -> Varchar,
        invitation -> Nullable<Uuid>,
        game_id -> Int8,
        created_at -> Nullable<Timestamptz>,
        updated_at -> Nullable<Timestamptz>,
    }
}

table! {
    slots (id) {
        id -> Int8,
        game_id -> Int8,
    }
}

joinable!(participants -> games (game_id));
joinable!(slots -> games (game_id));

allow_tables_to_appear_in_same_query!(
    games,
    participants,
    slots,
);
