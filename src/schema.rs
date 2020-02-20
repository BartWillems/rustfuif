table! {
    games (id) {
        id -> Int8,
        name -> Varchar,
        start_time -> Timestamptz,
        duration_in_seconds -> Nullable<Int4>,
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

table! {
    teams (id) {
        id -> Int8,
        name -> Varchar,
        invitation -> Nullable<Uuid>,
        game_id -> Int8,
        created_at -> Nullable<Timestamptz>,
        updated_at -> Nullable<Timestamptz>,
    }
}

joinable!(slots -> games (game_id));
joinable!(teams -> games (game_id));

allow_tables_to_appear_in_same_query!(games, slots, teams,);
