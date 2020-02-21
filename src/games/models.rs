use actix_web::Result;
use diesel::prelude::*;
use chrono::Duration;
use chrono::{DateTime, Utc};

use crate::db;
use crate::errors::ServiceError;
use crate::schema::games;

#[derive(Debug, Serialize, Deserialize, Queryable)]
pub struct Game {
    pub id: i64,
    pub name: String,
    pub start_time: DateTime<Utc>,
    pub close_time: DateTime<Utc>,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize, Insertable)]
#[table_name = "games"]
pub struct CreateGame {
    pub name: String,
    pub start_time: DateTime<Utc>,
    pub close_time: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct GameQuery {
    pub active: Option<bool>,
}

impl Game {
    pub fn create(new_game: CreateGame, conn: &db::Conn) -> Result<Game, ServiceError> {
        Game::check_duration(new_game.start_time, new_game.close_time)?;

        let game = diesel::insert_into(games::table)
            .values(&new_game)
            .get_result(conn)?;

        Ok(game)
    }

    fn check_duration(start_time: DateTime<Utc>, close_time: DateTime<Utc>) -> Result<(), ServiceError> {
        let duration: Duration = close_time.signed_duration_since(start_time);

        if duration.num_minutes() <= 0 {
            return Err(ServiceError::BadRequest(String::from("this game has not gone on long enough")));
        }
        Ok(())
    }

    pub fn find(game_id: i64, conn: &db::Conn) -> Result<Option<Game>, ServiceError> {
        let game = games::table
            .filter(games::id.eq(game_id))
            .first(conn)
            .optional()?;

        Ok(game)
    }

    pub fn load(conn: &db::Conn) -> Result<Vec<Game>, ServiceError> {
        let games = games::table.order(games::id).load::<Game>(conn)?;
        Ok(games)
    }

    pub fn load_active(conn: &db::Conn) -> Result<Vec<Game>, ServiceError> {
        let games = games::table
            .filter(games::close_time.gt(diesel::dsl::now))
            .order(games::id)
            .load::<Game>(conn)?;

        Ok(games)
    }
}


