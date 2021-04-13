use actix_web::Result;
use chrono::Duration;
use chrono::{DateTime, Utc};
use sqlx::{Pool, Postgres};
use url::Url;

use crate::errors::ServiceError;
use crate::invitations::{NewInvitation, State};
use crate::transactions::models::SalesCount;
use crate::users::{User, UserResponse};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
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

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
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
#[serde(rename_all = "camelCase")]
pub struct GameFilter {
    /// filter these games by %name%
    pub name: Option<String>,
    /// default false, set to true to hide games from the past
    pub completed: Option<bool>,
    /// list games created by a specific user
    pub owner_id: Option<i64>,
}

/// A GameUser is a user who is invited for a game
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GameUser {
    pub user_id: i64,
    pub username: String,
    pub invitation_state: String,
}

#[derive(Debug, Serialize, Queryable)]
#[serde(rename_all = "camelCase")]
pub struct GameResponse {
    pub id: i64,
    pub name: String,
    pub start_time: DateTime<Utc>,
    pub close_time: DateTime<Utc>,
    pub beverage_count: i16,
    pub owner: UserResponse,
}


/// minimum duration is 30 minutes
const MIN_GAME_SECONDS: i64 = 60 * 30;
/// maximum duration is 24 hours
const MAX_GAME_SECONDS: i64 = 60 * 60 * 24;

impl Game {
    /// Creates a new game, saves it in the database and automatically invites and
    /// accepts the creator in a transaction.
    ///
    /// When something fails, the transaction rolls-back, returns an error
    /// and nothing will have happened.
    #[tracing::instrument(name = "game::create")]
    pub async fn create(new_game: CreateGame, db: &Pool<Postgres>) -> Result<Game, ServiceError> {
        let transaction = db.begin().await?;

        let game: Game = sqlx::query_as!(
            Game,
            r#"
            INSERT INTO games (name, owner_id, start_time, close_time, beverage_count)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING *;
            "#,
            new_game.name,
            new_game.owner_id,
            new_game.start_time,
            new_game.close_time,
            new_game.beverage_count
        )
        .fetch_one(db)
        .await?;

        NewInvitation::new(game.id, game.owner_id)
            .accept()
            .save(db)
            .await?;

        SalesCount::initialize_slots(&game, db).await?;

        transaction.commit().await?;

        Ok(game)
    }

    #[tracing::instrument(name = "game::available_for_purchases")]
    pub async fn available_for_purchases(game_id: i64, user_id: i64, db: &Pool<Postgres>) -> Result<bool, ServiceError> {
        let game = sqlx::query!(r#"
            SELECT games.id
            FROM (games INNER JOIN invitations ON invitations.game_id = games.id) 
            WHERE games.id = $1 AND invitations.user_id = $2 AND invitations.state = $3 AND games.start_time < NOW() AND games.close_time > NOW()"#,
            game_id, user_id, State::ACCEPTED.to_string()
        ).fetch_optional(db).await?;

        Ok(game.is_some())
    }

    /// return the amount of active games at the moment
    #[tracing::instrument]
    pub async fn active_game_count(db: &Pool<Postgres>) -> Result<i64, sqlx::Error> {
        let res = sqlx::query!(r#"SELECT COUNT(*) as "count!" FROM games WHERE start_time < NOW() AND close_time > NOW()"#).fetch_one(db).await?;

        Ok(res.count)
    }

    /// return the total amount of created games
    #[tracing::instrument(name = "game::count")]
    pub async fn count(db: &Pool<Postgres>) -> Result<i64, sqlx::Error> {
        let res = sqlx::query!(r#"SELECT COUNT(*) as "count!" FROM games"#).fetch_one(db).await?;

        Ok(res.count)
    }

    #[tracing::instrument]
    pub async fn active_games(db: &Pool<Postgres>) -> Result<Vec<Game>, sqlx::Error> {
        sqlx::query_as!(Game, "SELECT * FROM games WHERE start_time < NOW() AND close_time > NOW()").fetch_all(db).await
    }

    #[tracing::instrument(name = "game::invite_user")]
    pub async fn invite_user(&self, user_id: i64, db: &Pool<Postgres>) -> Result<(), sqlx::Error> {
        NewInvitation::new(self.id, user_id).save(db).await?;

        Ok(())
    }

    #[tracing::instrument(name = "game::find_by_id")]
    pub async fn find_by_id(id: i64, db: &Pool<Postgres>) -> Result<Game, sqlx::Error> {
        let game = sqlx::query_as!(Game, "SELECT * FROM games WHERE id = $1", id)
            .fetch_one(db)
            .await?;

        Ok(game)
    }

    #[tracing::instrument(name = "game::find_all")]
    pub async fn find_all(
        filter: GameFilter,
        db: &Pool<Postgres>,
    ) -> Result<Vec<GameResponse>, sqlx::Error> {

        // Show only games that are in progress
        if !filter.completed.unwrap_or(true) {
            return sqlx::query_as!(
                GameResponse,
                r#"SELECT games.id, games.name, games.start_time, games.close_time, games.beverage_count, (users.id, users.username) as "owner!: UserResponse"
                FROM (games INNER JOIN users ON games.owner_id = users.id)
                WHERE games.close_time > NOW()
                ORDER BY games.start_time DESC"#
            ).fetch_all(db).await;
        }

        sqlx::query_as!(
            GameResponse,
            r#"SELECT games.id, games.name, games.start_time, games.close_time, games.beverage_count, (users.id, users.username) as "owner!: UserResponse"
            FROM (games INNER JOIN users ON games.owner_id = users.id)
            ORDER BY games.start_time DESC"#
        ).fetch_all(db).await
    }

    #[tracing::instrument(name = "game::find_by_user")]
    pub async fn find_by_user(
        user_id: i64,
        filter: GameFilter,
        db: &Pool<Postgres>,
    ) -> Result<Vec<GameResponse>, sqlx::Error> {
        // Show only games that are in progress
        if !filter.completed.unwrap_or(true) {
            return sqlx::query_as!(
                GameResponse,
                r#"SELECT games.id, games.name, games.start_time, games.close_time, games.beverage_count, (users.id, users.username) as "owner!: UserResponse"
                FROM (games INNER JOIN users ON games.owner_id = users.id)
                WHERE games.id IN (
                    SELECT game_id FROM invitations WHERE user_id = $1 AND state = $2
                ) AND games.close_time > NOW()
                ORDER BY games.start_time DESC"#,
                user_id, State::ACCEPTED.to_string()
            ).fetch_all(db).await;
        }

        let games = sqlx::query_as!(
            GameResponse,
            r#"SELECT games.id, games.name, games.start_time, games.close_time, games.beverage_count, (users.id, users.username) as "owner!: UserResponse"
            FROM (games INNER JOIN users ON games.owner_id = users.id)
            WHERE games.id IN (
                SELECT game_id FROM invitations WHERE user_id = $1 AND state = $2
            )
            ORDER BY games.start_time DESC"#,
            user_id, State::ACCEPTED.to_string()
        ).fetch_all(db).await?;

        Ok(games)
    }

    /// returns a list of users who have been invited for a game
    #[tracing::instrument(name = "Game::invited_users")]
    pub async fn invited_users(
        game_id: i64,
        db: &Pool<Postgres>,
    ) -> Result<Vec<GameUser>, sqlx::Error> {
        sqlx::query_as!(
            GameUser,
            r#"SELECT users.id as "user_id", username, invitations.state as "invitation_state"
            FROM users
            INNER JOIN invitations ON invitations.user_id = users.id
            WHERE invitations.game_id = $1"#, 
            game_id
        ).fetch_all(db).await        
    }

    /// Returns a list of users who have not yet been invited for a game
    #[tracing::instrument(name = "Game::find_available_users")]
    pub async fn find_available_users(game_id: i64, db: &Pool<Postgres>) -> Result<Vec<UserResponse>, sqlx::Error> {
        sqlx::query_as!(UserResponse, "SELECT id, username FROM users WHERE id NOT IN (SELECT user_id FROM invitations WHERE game_id = $1)", game_id).fetch_all(db).await
    }

    /// validates if a user is actually partaking in a game (invited and accepted)
    #[tracing::instrument(name = "Game::verify_user_participation")]
    pub async fn verify_user_participation(
        game_id: i64,
        user_id: i64,
        db: &Pool<Postgres>,
    ) -> Result<bool, ServiceError> {
        let row = sqlx::query!(
            r#"
            SELECT user_id
            FROM invitations
            WHERE game_id = $1 AND user_id = $2 AND state = $3
            "#,
            game_id,
            user_id,
            State::ACCEPTED.to_string()
        )
        .fetch_optional(db)
        .await?;

        Ok(row.is_some())
    }

    /// returns true if a user is an admin or created the game
    pub const fn is_owner(&self, user: &User) -> bool {
        user.is_admin || user.id == self.owner_id
    }

    #[tracing::instrument(name = "Game::update")]
    pub async fn update(&self, db: &Pool<Postgres>) -> Result<Game, sqlx::Error> {
        let game = sqlx::query_as!(
            Game,
            "UPDATE games SET name = $1 WHERE id = $2 RETURNING *",
            self.name,
            self.id
        )
        .fetch_one(db)
        .await?;

        Ok(game)
    }

    #[tracing::instrument(name = "Game::delete")]
    pub async fn delete(&self, db: &Pool<Postgres>) -> Result<(), ServiceError> {
        sqlx::query!("DELETE FROM games WHERE id = $1", self.id)
            .execute(db)
            .await?;

        Ok(())
    }

    #[tracing::instrument(name = "Game::get_beverages")]
    pub async fn get_beverages(&self, db: &Pool<Postgres>) -> Result<Vec<Beverage>, sqlx::Error> {
        Beverage::find_by_game(self.id, db).await
    }

    /// Update the prices for a game, returning the updated beverages
    #[tracing::instrument]
    pub async fn update_prices(&self, db: &Pool<Postgres>) -> Result<Vec<Beverage>, sqlx::Error> {
        let mut beverages = self.get_beverages(db).await?;

        let sales = SalesCount::find_by_game_for_update(self.id, db).await?;
        let average_sales = SalesCount::average_sales(&sales);

        for beverage in &mut beverages {
            for sale in &sales {
                if sale.slot_no != beverage.slot_no {
                    continue;
                }

                debug!("game({}) - beverage: {}", self.id, beverage.name);
                assert_eq!(sale.slot_no, beverage.slot_no);
                let offset = sale.get_offset(average_sales);
                let price = beverage.calculate_price(offset);
                debug!("setting price to: {}", price);
                beverage.set_price(price);
                beverage.save_price(db).await?;
            }
        }

        Ok(beverages)
    }

    /// Set all beverages for this game to their lowest possible value, returning the updated beverages
    #[tracing::instrument]
    pub async fn crash_prices(&self, db: &Pool<Postgres>) -> Result<Vec<Beverage>, sqlx::Error> {
        let mut beverages = self.get_beverages(db).await?;

        for beverage in &mut beverages {
            beverage.set_price(beverage.min_price);
            beverage.save_price(db).await?;
        }

        Ok(beverages)
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

        if self.name.trim().is_empty() {
            bad_request!("name is too short");
        }

        if self.name.trim().len() > 40 {
            bad_request!("name is too long, maximum 40 characters");
        }

        if self.beverage_count < 2 {
            bad_request!("at least 2 beverages should be used");
        }

        if self.beverage_count > 16 {
            bad_request!("maximum 16 different beverages allowed");
        }

        Ok(())
    }
}

#[derive(Deserialize, Serialize, Debug)]
#[serde(rename_all = "camelCase")]
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

    #[serde(skip_deserializing)]
    pub current_price: i64,
}

impl Beverage {
    pub async fn save(&self, db: &Pool<Postgres>) -> Result<Beverage, ServiceError> {
        let game = Game::find_by_id(self.game_id, db).await?;

        if self.slot_no >= game.beverage_count {
            bad_request!("a beverage slot exceeds the maximum configured beverage slots");
        }

        let beverage = sqlx::query_as!(Beverage, r#"
            INSERT INTO beverages (game_id, user_id, slot_no, name, image_url, min_price, max_price, starting_price, current_price)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            RETURNING *"#, 
            self.game_id, self.user_id, self.slot_no, self.name, self.image_url, self.min_price, self.max_price, self.starting_price, self.current_price
        ).fetch_one(db).await?;

        Ok(beverage)
    }

    pub async fn find(
        game_id: i64,
        user_id: i64,
        db: &Pool<Postgres>,
    ) -> Result<Vec<Beverage>, sqlx::Error> {
        sqlx::query_as!(
            Beverage, 
            r#"
            SELECT * FROM beverages
            WHERE user_id = $1 AND game_id = $2
            ORDER BY slot_no
            "#,
            user_id, game_id
        ).fetch_all(db).await
    }

    pub async fn find_by_game(
        game_id: i64,
        db: &Pool<Postgres>,
    ) -> Result<Vec<Beverage>, sqlx::Error> {
        sqlx::query_as!(
            Beverage,
            "SELECT * FROM beverages WHERE game_id = $1 ORDER BY slot_no",
            game_id
        )
        .fetch_all(db)
        .await
    }

    pub async fn update(&self, db: &Pool<Postgres>) -> Result<Beverage, sqlx::Error> {
        sqlx::query_as!(
            Beverage,
            r#"
            UPDATE beverages
            SET name = $1, image_url = $2, min_price = $3, max_price = $4, starting_price = $5
            WHERE slot_no = $6 AND game_id = $7 AND user_id = $8
            RETURNING *
            "#,
            self.name,
            self.image_url,
            self.min_price,
            self.max_price,
            self.starting_price,
            self.slot_no,
            self.game_id,
            self.user_id
        )
        .fetch_one(db)
        .await
    }

    #[tracing::instrument(name = "Beverage::save_price")]
    pub async fn save_price(&self, db: &Pool<Postgres>) -> Result<Beverage, sqlx::Error> {
        sqlx::query_as!(
            Beverage, 
            "UPDATE beverages SET current_price = $1 WHERE game_id = $2 AND user_id = $3 AND slot_no = $4 RETURNING *", 
            self.price(), self.game_id, self.user_id, self.slot_no
        ).fetch_one(db).await
    }

    /// calculate the price of a beverage based on it's offset from the average sales
    pub const fn calculate_price(&self, offset: i64) -> i64 {
        let price = self.starting_price + offset * (self.starting_price / 20);

        if price > self.max_price {
            return self.max_price;
        } else if price < self.min_price {
            return self.min_price;
        }

        // round to 10 cents
        let mod_ten = price % 10;
        if mod_ten >= 5 {
            price + (10 - mod_ten)
        } else {
            price - mod_ten
        }
    }

    /// set the current price
    pub fn set_price(&mut self, price: i64) {
        self.current_price = price;
    }

    /// get the current price
    pub fn price(&self) -> i64 {
        self.current_price
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

        if self.name.trim().is_empty() {
            bad_request!("name is too short");
        }

        if self.name.trim().len() > 40 {
            bad_request!("name is too long, maximum 40 characters");
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
            current_price: 250,
        };

        assert!(beverage.calculate_price(500) <= beverage.max_price);
        assert!(beverage.calculate_price(-500) >= beverage.min_price);
    }

    #[test]
    fn valid_beverage_count_range() {
        let start_time: DateTime<Utc> = Utc::now().add(Duration::days(1));
        let close_time = start_time.add(Duration::hours(1));
        let mut game = CreateGame {
            owner_id: 1,
            beverage_count: -1,
            name: String::from("some game"),
            start_time: start_time,
            close_time: close_time,
        };

        assert!(Validator::new(game.clone()).validate().is_err());

        game.beverage_count = 0;
        assert!(Validator::new(game.clone()).validate().is_err());

        game.beverage_count = 1;
        assert!(Validator::new(game.clone()).validate().is_err());

        game.beverage_count = 2;
        assert!(Validator::new(game.clone()).validate().is_ok());
    }
}
