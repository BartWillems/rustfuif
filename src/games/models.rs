use actix_web::Result;
use chrono::Duration;
use chrono::{DateTime, Utc};
use diesel::prelude::*;

use crate::db;
use crate::errors::ServiceError;
use crate::schema::games;

#[derive(Debug, Serialize, Deserialize, Queryable, Identifiable, AsChangeset)]
pub struct Game {
    pub id: i64,
    pub name: String,
    pub owner_id: i64,
    pub start_time: DateTime<Utc>,
    pub close_time: DateTime<Utc>,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize, Insertable)]
#[table_name = "games"]
pub struct CreateGame {
    pub name: String,
    #[serde(skip)]
    pub owner_id: i64,
    pub start_time: DateTime<Utc>,
    pub close_time: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct GameQuery {
    pub name: Option<String>,
    pub is_active: Option<bool>,
    pub owner_id: Option<i64>,
}

impl Game {
    pub fn create(new_game: CreateGame, conn: &db::Conn) -> Result<Game, ServiceError> {
        Game::check_duration(new_game.start_time, new_game.close_time)?;

        let game = diesel::insert_into(games::table)
            .values(&new_game)
            .get_result(conn)?;

        Ok(game)
    }

    fn check_duration(
        start_time: DateTime<Utc>,
        close_time: DateTime<Utc>,
    ) -> Result<(), ServiceError> {
        let duration: Duration = close_time.signed_duration_since(start_time);

        if duration.num_minutes() <= 0 {
            return Err(ServiceError::BadRequest(
                "this game has not gone on long enough".to_string(),
            ));
        }
        Ok(())
    }

    pub fn find_by_id(game_id: i64, conn: &db::Conn) -> Result<Game, ServiceError> {
        let game = games::table.filter(games::id.eq(game_id)).first(conn)?;
        Ok(game)
    }

    pub fn find_all(filter: GameQuery, conn: &db::Conn) -> Result<Vec<Game>, ServiceError> {
        let mut query = games::table.into_boxed();

        if filter.is_active.unwrap_or(false) {
            query = query.filter(games::close_time.gt(diesel::dsl::now));
        }

        if let Some(id) = filter.owner_id {
            query = query.filter(games::owner_id.eq(id));
        }

        if let Some(name) = filter.name {
            query = query.filter(games::name.like(format!("%{}%", name)));
        }

        let games = query.load::<Game>(conn)?;
        Ok(games)
    }

    pub fn update(&self, conn: &db::Conn) -> Result<Game, ServiceError> {
        let game = diesel::update(self).set(self).get_result(conn)?;

        Ok(game)
    }

    pub fn delete(&self, conn: &db::Conn) -> Result<(), ServiceError> {
        diesel::delete(self).execute(conn)?;

        Ok(())
    }

    pub fn delete_by_id(game_id: i64, conn: &db::Conn) -> Result<(), ServiceError> {
        diesel::delete(games::table.filter(games::id.eq(game_id))).execute(conn)?;

        Ok(())
    }
}
