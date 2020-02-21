use actix_web::web::{Data, HttpResponse, Json, Path, Query};
use actix_web::{get, post, Result};
use diesel::prelude::*;

use crate::db;
use crate::errors::ServiceError;
use crate::schema::games;
use crate::web;

#[derive(Debug, Serialize, Deserialize, Queryable)]
pub struct Game {
    pub id: i64,
    pub name: String,
    pub start_time: chrono::NaiveDateTime,
    pub close_time: chrono::NaiveDateTime,
    pub created_at: Option<chrono::NaiveDateTime>,
    pub updated_at: Option<chrono::NaiveDateTime>,
}

#[derive(Debug, Deserialize, Insertable)]
#[table_name = "games"]
pub struct CreateGame {
    pub name: String,
    pub start_time: chrono::NaiveDateTime,
    pub close_time: chrono::NaiveDateTime,
}

#[derive(Debug, Deserialize)]
pub struct GameQuery {
    pub active: Option<bool>,
}

impl Game {
    pub fn create(new_game: CreateGame, conn: &db::Conn) -> Result<Game, ServiceError> {
        let game = diesel::insert_into(games::table)
            .values(&new_game)
            .get_result(conn)?;

        Ok(game)
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

#[get("/")]
pub async fn get_games(query: Query<GameQuery>, pool: Data<db::Pool>) -> web::Response {
    let conn = pool.get()?;

    debug!("active: {:#?}", query);

    let games: Vec<Game>;
    if query.active.unwrap_or(false) {
        games = Game::load_active(&conn)?;
    } else {
        games = Game::load(&conn)?;
    }

    Ok(HttpResponse::Ok().json(games))
}

#[post("/")]
pub async fn create_game(game: Json<CreateGame>, pool: Data<db::Pool>) -> web::Response {
    let conn = pool.get()?;

    let game = Game::create(game.into_inner(), &conn)?;

    Ok(HttpResponse::Ok().json(game))
}

#[post("/{id}")]
pub async fn update_game(_game: Json<Game>, _pool: Data<db::Pool>) -> web::Response {
    unimplemented!();
}

#[get("/{id}")]
pub async fn get_game(game_id: Path<i64>, pool: Data<db::Pool>) -> web::Response {
    let conn = pool.get()?;

    match Game::find(*game_id, &conn)? {
        Some(game) => return Ok(HttpResponse::Ok().json(game)),
        None => Ok(HttpResponse::NotFound().json(format!("game {} not found", game_id))),
    }
}
