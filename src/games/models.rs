use actix_web::Result;
use chrono::Duration;
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use regex::Regex;
use std::collections::HashMap;
use url::Url;

use crate::cache;
use crate::db;
use crate::errors::ServiceError;
use crate::invitations::{InvitationQuery, NewInvitation, State};
use crate::schema::{beverages, games, invitations, users};
use crate::transactions::models::SalesCount;
use crate::users::{User, UserResponse};

#[derive(Debug, Serialize, Deserialize, Queryable, Identifiable, AsChangeset)]
pub struct Game {
    pub id: i64,
    pub name: String,
    pub owner_id: i64,
    pub start_time: DateTime<Utc>,
    pub close_time: DateTime<Utc>,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    pub beverage_count: i16,
}

#[derive(Debug, Clone, Deserialize, Insertable)]
#[table_name = "games"]
pub struct CreateGame {
    pub name: String,
    #[serde(skip)]
    pub owner_id: i64,
    pub start_time: DateTime<Utc>,
    pub close_time: DateTime<Utc>,
    pub beverage_count: i16,
}

/// GameFilter a struct that the client
/// can use to query for games.
#[derive(Debug, Deserialize)]
pub struct GameFilter {
    /// filter these games by %name%
    pub name: Option<String>,
    /// default false, set to true to hide games from the past
    pub completed: Option<bool>,
    /// list games created by a specific user
    pub owner_id: Option<i64>,
}

/// A GameUser is a user who is invited for a game
#[derive(Serialize, Queryable)]
pub struct GameUser {
    pub user_id: i64,
    pub username: String,
    pub invitation_state: String,
}

#[derive(Serialize, Queryable)]
pub struct GameResponse {
    pub id: i64,
    pub name: String,
    pub start_time: DateTime<Utc>,
    pub close_time: DateTime<Utc>,
    pub beverage_count: i16,
    pub owner: crate::users::UserResponse,
}

/// minimum duration is 30 minutes
const MIN_GAME_SECONDS: i64 = 60 * 30;
/// maximum duration is 24 hours
const MAX_GAME_SECONDS: i64 = 60 * 60 * 24;

const PRICE_MULTIPLIER: i64 = 10;

impl Game {
    /// Creates a new game, saves it in the database and automatically invites and
    /// accepts the creator in a transaction.
    ///
    /// When something fails, the transaction rolls-back, returns an error
    /// and nothing will have happened.
    pub fn create(new_game: CreateGame, conn: &db::Conn) -> Result<Game, ServiceError> {
        let game = conn.transaction::<Game, diesel::result::Error, _>(|| {
            let game: Game = diesel::insert_into(games::table)
                .values(&new_game)
                .get_result(conn)?;

            NewInvitation::new(game.id, game.owner_id)
                .accept()
                .save(conn)?;

            SalesCount::initialize_slots(&game, conn)?;

            Ok(game)
        })?;

        Ok(game)
    }

    pub fn is_open(game_id: i64, user_id: i64, conn: &db::Conn) -> Result<bool, ServiceError> {
        use diesel::dsl::now;

        let game_id = games::table
            .inner_join(invitations::table)
            .filter(games::id.eq(game_id))
            .filter(invitations::user_id.eq(user_id))
            .filter(invitations::state.eq(State::ACCEPTED.to_string()))
            .filter(games::start_time.lt(now))
            .filter(games::close_time.gt(now))
            .select(games::id)
            .first::<i64>(conn)
            .optional()?;
        Ok(game_id.is_some())
    }

    /// return the amount of active games at the moment
    pub fn active_games(conn: &db::Conn) -> Result<i64, ServiceError> {
        use diesel::dsl::{now, sql};

        let count = games::table
            .filter(games::start_time.lt(now))
            .filter(games::close_time.gt(now))
            .select(sql::<diesel::sql_types::BigInt>("COUNT(*)"))
            .first::<i64>(conn)?;

        Ok(count)
    }

    pub fn invite_user(&self, user_id: i64, conn: &db::Conn) -> Result<(), ServiceError> {
        let invite = NewInvitation::new(self.id, user_id);
        invite.save(conn)?;
        Ok(())
    }

    pub fn find_by_id(id: i64, conn: &db::Conn) -> Result<Game, ServiceError> {
        if let Some(game) = cache::find(id).unwrap_or(None) {
            return Ok(game);
        }

        let game = games::table.filter(games::id.eq(id)).first::<Game>(conn)?;

        if let Err(e) = cache::set(&game, game.id) {
            error!("unable to cache game: {}", e);
        }

        Ok(game)
    }

    pub fn find_all(
        filter: GameFilter,
        conn: &db::Conn,
    ) -> Result<Vec<GameResponse>, ServiceError> {
        let mut query = games::table
            .inner_join(users::table)
            .select((
                games::id,
                games::name,
                games::start_time,
                games::close_time,
                games::beverage_count,
                (users::id, users::username),
            ))
            .order(games::start_time)
            .into_boxed();

        if !filter.completed.unwrap_or(true) {
            query = query.filter(games::close_time.gt(diesel::dsl::now));
        }

        if let Some(id) = filter.owner_id {
            query = query.filter(games::owner_id.eq(id));
        }

        if let Some(name) = filter.name {
            query = query.filter(games::name.ilike(format!("%{}%", name)));
        }

        let games = query.load::<GameResponse>(conn)?;
        Ok(games)
    }

    pub fn find_by_user(
        user_id: i64,
        filter: GameFilter,
        conn: &db::Conn,
    ) -> Result<Vec<GameResponse>, ServiceError> {
        let invitations = invitations::table
            .filter(invitations::user_id.eq(user_id))
            .filter(invitations::state.eq(State::ACCEPTED.to_string()))
            .select(invitations::game_id);

        let mut query = games::table
            .inner_join(users::table)
            .select((
                games::id,
                games::name,
                games::start_time,
                games::close_time,
                games::beverage_count,
                (users::id, users::username),
            ))
            .into_boxed();

        use diesel::dsl::any;

        if !filter.completed.unwrap_or(true) {
            query = query.filter(games::close_time.gt(diesel::dsl::now));
        }

        if let Some(id) = filter.owner_id {
            query = query.filter(games::owner_id.eq(id));
        }

        if let Some(name) = filter.name {
            query = query.filter(games::name.ilike(format!("%{}%", name)));
        }

        let games = query
            .filter(games::id.eq(any(invitations)))
            .load::<GameResponse>(conn)?;
        Ok(games)
    }

    /// returns a list of users who have been invited for a game
    /// filter by changing the invitation state
    pub fn find_users(
        game_id: i64,
        filter: InvitationQuery,
        conn: &db::Conn,
    ) -> Result<Vec<GameUser>, ServiceError> {
        let mut query = invitations::table
            .inner_join(users::table)
            .filter(invitations::game_id.eq(game_id))
            .into_boxed();

        if let Some(state) = filter.state {
            query = query.filter(invitations::state.eq(state.to_string()));
        }

        let users = query
            .select((users::id, users::username, invitations::state))
            .load::<GameUser>(conn)?;

        Ok(users)
    }

    /// show users who are not yet invited in a game
    pub fn find_available_users(
        game_id: i64,
        conn: &db::Conn,
    ) -> Result<Vec<UserResponse>, ServiceError> {
        let participants = invitations::table
            .select(invitations::user_id)
            .filter(invitations::game_id.eq(game_id));

        let users = users::table
            .select((users::id, users::username))
            .filter(users::id.ne_all(participants))
            .load::<UserResponse>(conn)?;

        Ok(users)
    }

    /// validates if a user is actually partaking in a game (invited and accepted)
    pub fn verify_user(game_id: i64, user_id: i64, conn: &db::Conn) -> Result<bool, ServiceError> {
        let res = invitations::table
            .filter(invitations::game_id.eq(game_id))
            .filter(invitations::user_id.eq(user_id))
            .filter(invitations::state.eq(State::ACCEPTED.to_string()))
            .select(invitations::user_id)
            .first::<i64>(conn)
            .optional()?;

        Ok(res.is_some())
    }

    /// returns true if a user is an admin or created the game
    pub const fn is_owner(&self, user: &User) -> bool {
        user.is_admin || user.id == self.owner_id
    }

    pub fn update(&self, conn: &db::Conn) -> Result<Game, ServiceError> {
        let game: Game = diesel::update(self).set(self).get_result(conn)?;

        if let Err(e) = cache::set(&game, game.id) {
            error!("unable to cache game: {}", e);
        }

        Ok(game)
    }

    pub fn delete(&self, conn: &db::Conn) -> Result<(), ServiceError> {
        diesel::delete(self).execute(conn)?;

        cache::delete(format!("game.{}", self.id))?;

        Ok(())
    }

    pub fn delete_by_id(game_id: i64, conn: &db::Conn) -> Result<(), ServiceError> {
        diesel::delete(games::table.filter(games::id.eq(game_id))).execute(conn)?;

        Ok(())
    }

    /// returns if the game is going on at the moment
    /// Should perhaps be changed to use database values
    /// should be used to determine if the start/ending times could be altered
    pub fn is_in_progress(&self) -> bool {
        let now = chrono::offset::Utc::now();
        if self.start_time < now && self.close_time > now {
            return true;
        }
        false
    }

    pub fn prices(
        game_id: i64,
        user_id: i64,
        conn: &db::Conn,
    ) -> Result<HashMap<i16, i64>, ServiceError> {
        let mut prices: HashMap<i16, i64> = HashMap::new();
        let offsets = SalesCount::get_price_offsets(game_id, conn)?;
        let beverages = Beverage::find(game_id, user_id, conn)?;

        for beverage in &beverages {
            if let Some(offset) = offsets.get(&beverage.slot_no) {
                prices.insert(beverage.slot_no, beverage.calculate_price(*offset));
            }
        }

        Ok(prices)
    }
}

impl crate::cache::Cache for Game {
    fn cache_key<T: std::fmt::Display>(id: T) -> String {
        format!("game.{}", id)
    }
}

impl crate::validator::Validate<CreateGame> for CreateGame {
    fn validate(&self) -> Result<(), ServiceError> {
        if self.start_time <= Utc::now() {
            bad_request!("the game can't start in the past");
        }

        let duration: Duration = self.close_time.signed_duration_since(self.start_time);
        if duration.num_seconds() < MIN_GAME_SECONDS {
            bad_request!("this game has not gone on long enough, minimum duration is 30 minutes");
        }

        if duration.num_seconds() > MAX_GAME_SECONDS {
            bad_request!("the max duration of a game is 24 hours");
        }

        let pattern: Regex = Regex::new(r"^[a-zA-Z0-9_-]+( [a-zA-Z0-9_]+)*$").unwrap();

        if self.name.trim().is_empty() {
            bad_request!("name is too short");
        }

        if self.name.trim().len() > 40 {
            bad_request!("name is too long, maximum 40 characters");
        }

        if !pattern.is_match(&self.name) {
            bad_request!("name can only contain letters, numbers, spaces, '-' and '_'");
        }

        if self.beverage_count > 16 {
            bad_request!("maximum 16 different beverages allowed");
        }

        Ok(())
    }
}

#[derive(Insertable, Deserialize, Serialize, Queryable, Debug)]
pub struct Beverage {
    #[serde(skip_deserializing)]
    pub game_id: i64,

    #[serde(skip_deserializing)]
    pub user_id: i64,

    pub slot_no: i16,
    pub name: String,
    pub image_url: Option<String>,
    pub min_price: i64,
    pub max_price: i64,
    pub starting_price: i64,
}

impl Beverage {
    pub fn save(&self, conn: &db::Conn) -> Result<Beverage, ServiceError> {
        let game = Game::find_by_id(self.game_id, conn)?;

        if self.slot_no >= game.beverage_count {
            bad_request!("a beverage slot exceeds the maximum configured beverage slots");
        }

        let config = diesel::insert_into(beverages::table)
            .values(self)
            .get_result::<Beverage>(conn)?;

        let _ = cache::delete(format!("beverage_config.{}.{}", self.game_id, self.user_id))
            .map_err(|e| {
                error!("unable to delete beverage config from cache: {}", e);
            });

        Ok(config)
    }

    pub fn find(
        game_id: i64,
        user_id: i64,
        conn: &db::Conn,
    ) -> Result<Vec<Beverage>, ServiceError> {
        if let Some(configs) = cache::find(format!("{}.{}", game_id, user_id)).unwrap_or(None) {
            return Ok(configs);
        }

        let configs = beverages::table
            .filter(beverages::user_id.eq(user_id))
            .filter(beverages::game_id.eq(game_id))
            .order(beverages::slot_no)
            .load::<Beverage>(conn)?;

        if let Err(e) = cache::set(&configs, format!("{}.{}", game_id, user_id)) {
            error!("unable to cache beverage config: {}", e);
        }

        Ok(configs)
    }

    pub fn update(&self, conn: &db::Conn) -> Result<Beverage, ServiceError> {
        use crate::schema::beverages::dsl::*;

        let config = diesel::update(beverages)
            .filter(slot_no.eq(self.slot_no))
            .filter(game_id.eq(self.game_id))
            .filter(user_id.eq(self.user_id))
            .set((
                name.eq(self.name.clone()),
                image_url.eq(self.image_url.clone()),
                min_price.eq(self.min_price),
                max_price.eq(self.max_price),
                starting_price.eq(self.starting_price),
            ))
            .get_result::<Beverage>(conn)?;

        let _ = cache::delete(format!("beverage_config.{}.{}", self.game_id, self.user_id))
            .map_err(|e| {
                error!("unable to delete beverage config from cache: {}", e);
            });

        Ok(config)
    }

    pub const fn calculate_price(&self, offset: i64) -> i64 {
        // TODO: be able to configure the multiplier
        let price = self.starting_price + offset * PRICE_MULTIPLIER;

        if price > self.max_price {
            return self.max_price;
        } else if price < self.min_price {
            return self.min_price;
        }

        price
    }
}

impl crate::cache::Cache for Vec<Beverage> {
    fn cache_key<T: std::fmt::Display>(id: T) -> String {
        format!("beverage_config.{}", id)
    }
}

impl crate::validator::Validate<Beverage> for Beverage {
    fn validate(&self) -> Result<(), ServiceError> {
        if self.slot_no < 0 {
            bad_request!("the slot number cannot be negative");
        }

        if self.min_price <= 0 {
            bad_request!("the minimum price has to be above 0");
        }

        if self.starting_price <= self.min_price {
            bad_request!("the starting price should be bigger than the minimum price");
        }

        if self.max_price <= self.starting_price {
            bad_request!("the the maximum price should be bigger than the starting price");
        }

        if let Some(url) = self.image_url.as_ref() {
            if Url::parse(&url).is_err() {
                bad_request!("the image url is not a valid url");
            }
        }

        let pattern: Regex = Regex::new(r"^[a-zA-Z0-9_]+( [a-zA-Z0-9_]+)*$").unwrap();

        if self.name.trim().is_empty() {
            bad_request!("name is too short");
        }

        if self.name.trim().len() > 40 {
            bad_request!("name is too long, maximum 40 characters");
        }

        if !pattern.is_match(&self.name) {
            bad_request!("name can only contain letters, numbers, spaces, '-' and '_'");
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::validator::Validator;
    use std::ops::Add;

    #[test]
    fn invalid_game_duration() {
        let time: DateTime<Utc> = Utc::now().add(Duration::days(1));

        let smaller_time = time.add(Duration::hours(-1));

        let game_with_same_times = CreateGame {
            name: String::from("some_name"),
            owner_id: 1,
            start_time: time,
            close_time: time,
            beverage_count: 8,
        };

        let game_with_smaller_end_time = CreateGame {
            name: String::from("some_name"),
            owner_id: 1,
            start_time: time,
            close_time: smaller_time,
            beverage_count: 8,
        };

        let game_with_equal_bigger_end_time = CreateGame {
            name: String::from("some_name"),
            owner_id: 1,
            start_time: smaller_time,
            close_time: time,
            beverage_count: 8,
        };

        assert!(Validator::new(game_with_same_times).validate().is_err());
        assert!(Validator::new(game_with_smaller_end_time)
            .validate()
            .is_err());

        assert!(Validator::new(game_with_equal_bigger_end_time)
            .validate()
            .is_ok());
    }

    #[test]
    fn valid_game_names() {
        let start_time: DateTime<Utc> = Utc::now().add(Duration::days(1));
        let close_time = start_time.add(Duration::hours(1));

        let mut game = CreateGame {
            name: String::from("some-game"),
            owner_id: 1,
            start_time,
            close_time,
            beverage_count: 8,
        };

        assert!(Validator::new(game.clone()).validate().is_ok());

        game.name = String::from("name with spaces");
        assert!(Validator::new(game.clone()).validate().is_ok());

        game.name = String::from("n4m3 with numb3rs");
        assert!(Validator::new(game.clone()).validate().is_ok());

        game.name = String::from("name-with_special-characters");
        assert!(Validator::new(game.clone()).validate().is_ok());
    }

    #[test]
    fn invalid_game_names() {
        let start_time: DateTime<Utc> = Utc::now().add(Duration::days(1));
        let close_time = start_time.add(Duration::hours(1));
        let mut game = CreateGame {
            name: String::from("some-game@"),
            owner_id: 1,
            start_time,
            close_time,
            beverage_count: 8,
        };

        assert!(Validator::new(game.clone()).validate().is_err());

        game.name = String::from("<html>");
        assert!(Validator::new(game.clone()).validate().is_err());

        game.name = String::from("('something')");
        assert!(Validator::new(game.clone()).validate().is_err());
    }

    #[test]
    fn beverage_price_range() {
        let beverage = Beverage {
            game_id: 1,
            name: String::from("Orval"),
            image_url: None,
            max_price: 500,
            min_price: 200,
            starting_price: 250,
            slot_no: 0,
            user_id: 0,
        };

        assert!(beverage.calculate_price(500) <= beverage.max_price);
        assert!(beverage.calculate_price(-500) >= beverage.min_price);
    }
}
