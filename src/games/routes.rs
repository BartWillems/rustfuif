use actix_web::web::{Data, HttpResponse, Json, Path, Query};
use actix_web::{get, post};

use crate::db;
use crate::web;

use crate::games::models::{Game, GameQuery, CreateGame};

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