table! {
    /// Representation of the `beverage_configs` table.
    ///
    /// (Automatically generated by Diesel.)
    beverage_configs (user_id, game_id, slot_no) {
        /// The `game_id` column of the `beverage_configs` table.
        ///
        /// Its SQL type is `Int8`.
        ///
        /// (Automatically generated by Diesel.)
        game_id -> Int8,
        /// The `user_id` column of the `beverage_configs` table.
        ///
        /// Its SQL type is `Int8`.
        ///
        /// (Automatically generated by Diesel.)
        user_id -> Int8,
        /// The `slot_no` column of the `beverage_configs` table.
        ///
        /// Its SQL type is `Int2`.
        ///
        /// (Automatically generated by Diesel.)
        slot_no -> Int2,
        /// The `name` column of the `beverage_configs` table.
        ///
        /// Its SQL type is `Varchar`.
        ///
        /// (Automatically generated by Diesel.)
        name -> Varchar,
        /// The `image_url` column of the `beverage_configs` table.
        ///
        /// Its SQL type is `Nullable<Varchar>`.
        ///
        /// (Automatically generated by Diesel.)
        image_url -> Nullable<Varchar>,
        /// The `min_price` column of the `beverage_configs` table.
        ///
        /// Its SQL type is `Int4`.
        ///
        /// (Automatically generated by Diesel.)
        min_price -> Int4,
        /// The `max_price` column of the `beverage_configs` table.
        ///
        /// Its SQL type is `Int4`.
        ///
        /// (Automatically generated by Diesel.)
        max_price -> Int4,
        /// The `starting_price` column of the `beverage_configs` table.
        ///
        /// Its SQL type is `Int4`.
        ///
        /// (Automatically generated by Diesel.)
        starting_price -> Int4,
    }
}

table! {
    /// Representation of the `games` table.
    ///
    /// (Automatically generated by Diesel.)
    games (id) {
        /// The `id` column of the `games` table.
        ///
        /// Its SQL type is `Int8`.
        ///
        /// (Automatically generated by Diesel.)
        id -> Int8,
        /// The `name` column of the `games` table.
        ///
        /// Its SQL type is `Varchar`.
        ///
        /// (Automatically generated by Diesel.)
        name -> Varchar,
        /// The `owner_id` column of the `games` table.
        ///
        /// Its SQL type is `Int8`.
        ///
        /// (Automatically generated by Diesel.)
        owner_id -> Int8,
        /// The `start_time` column of the `games` table.
        ///
        /// Its SQL type is `Timestamptz`.
        ///
        /// (Automatically generated by Diesel.)
        start_time -> Timestamptz,
        /// The `close_time` column of the `games` table.
        ///
        /// Its SQL type is `Timestamptz`.
        ///
        /// (Automatically generated by Diesel.)
        close_time -> Timestamptz,
        /// The `created_at` column of the `games` table.
        ///
        /// Its SQL type is `Nullable<Timestamptz>`.
        ///
        /// (Automatically generated by Diesel.)
        created_at -> Nullable<Timestamptz>,
        /// The `updated_at` column of the `games` table.
        ///
        /// Its SQL type is `Nullable<Timestamptz>`.
        ///
        /// (Automatically generated by Diesel.)
        updated_at -> Nullable<Timestamptz>,
    }
}

table! {
    /// Representation of the `invitations` table.
    ///
    /// (Automatically generated by Diesel.)
    invitations (id) {
        /// The `id` column of the `invitations` table.
        ///
        /// Its SQL type is `Int8`.
        ///
        /// (Automatically generated by Diesel.)
        id -> Int8,
        /// The `user_id` column of the `invitations` table.
        ///
        /// Its SQL type is `Int8`.
        ///
        /// (Automatically generated by Diesel.)
        user_id -> Int8,
        /// The `game_id` column of the `invitations` table.
        ///
        /// Its SQL type is `Int8`.
        ///
        /// (Automatically generated by Diesel.)
        game_id -> Int8,
        /// The `state` column of the `invitations` table.
        ///
        /// Its SQL type is `Varchar`.
        ///
        /// (Automatically generated by Diesel.)
        state -> Varchar,
        /// The `created_at` column of the `invitations` table.
        ///
        /// Its SQL type is `Nullable<Timestamptz>`.
        ///
        /// (Automatically generated by Diesel.)
        created_at -> Nullable<Timestamptz>,
        /// The `updated_at` column of the `invitations` table.
        ///
        /// Its SQL type is `Nullable<Timestamptz>`.
        ///
        /// (Automatically generated by Diesel.)
        updated_at -> Nullable<Timestamptz>,
    }
}

table! {
    /// Representation of the `transactions` table.
    ///
    /// (Automatically generated by Diesel.)
    transactions (id) {
        /// The `id` column of the `transactions` table.
        ///
        /// Its SQL type is `Int8`.
        ///
        /// (Automatically generated by Diesel.)
        id -> Int8,
        /// The `user_id` column of the `transactions` table.
        ///
        /// Its SQL type is `Int8`.
        ///
        /// (Automatically generated by Diesel.)
        user_id -> Int8,
        /// The `game_id` column of the `transactions` table.
        ///
        /// Its SQL type is `Int8`.
        ///
        /// (Automatically generated by Diesel.)
        game_id -> Int8,
        /// The `slot_no` column of the `transactions` table.
        ///
        /// Its SQL type is `Int2`.
        ///
        /// (Automatically generated by Diesel.)
        slot_no -> Int2,
        /// The `created_at` column of the `transactions` table.
        ///
        /// Its SQL type is `Nullable<Timestamptz>`.
        ///
        /// (Automatically generated by Diesel.)
        created_at -> Nullable<Timestamptz>,
    }
}

table! {
    /// Representation of the `users` table.
    ///
    /// (Automatically generated by Diesel.)
    users (id) {
        /// The `id` column of the `users` table.
        ///
        /// Its SQL type is `Int8`.
        ///
        /// (Automatically generated by Diesel.)
        id -> Int8,
        /// The `username` column of the `users` table.
        ///
        /// Its SQL type is `Varchar`.
        ///
        /// (Automatically generated by Diesel.)
        username -> Varchar,
        /// The `password` column of the `users` table.
        ///
        /// Its SQL type is `Varchar`.
        ///
        /// (Automatically generated by Diesel.)
        password -> Varchar,
        /// The `is_admin` column of the `users` table.
        ///
        /// Its SQL type is `Bool`.
        ///
        /// (Automatically generated by Diesel.)
        is_admin -> Bool,
        /// The `created_at` column of the `users` table.
        ///
        /// Its SQL type is `Nullable<Timestamptz>`.
        ///
        /// (Automatically generated by Diesel.)
        created_at -> Nullable<Timestamptz>,
        /// The `updated_at` column of the `users` table.
        ///
        /// Its SQL type is `Nullable<Timestamptz>`.
        ///
        /// (Automatically generated by Diesel.)
        updated_at -> Nullable<Timestamptz>,
    }
}

joinable!(beverage_configs -> games (game_id));
joinable!(beverage_configs -> users (user_id));
joinable!(games -> users (owner_id));
joinable!(invitations -> games (game_id));
joinable!(invitations -> users (user_id));
joinable!(transactions -> games (game_id));
joinable!(transactions -> users (user_id));

allow_tables_to_appear_in_same_query!(
    beverage_configs,
    games,
    invitations,
    transactions,
    users,
);
