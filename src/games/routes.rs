use actix_web::web;
use actix_web::web::{Data, HttpResponse, Json, Path, Query};
use actix_web::{get, post};

use crate::db;
use crate::errors::ServiceError;
use crate::server;

use crate::games::models::{CreateGame, Game, GameQuery};

// TODO: use web::block to offload blocking Diesel code without blocking server thread

#[get("/")]
pub async fn get_games(query: Query<GameQuery>, pool: Data<db::Pool>) -> server::Response {
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
pub async fn create_game(game: Json<CreateGame>, pool: Data<db::Pool>) -> server::Response {
    let conn = pool.get()?;

    let game = Game::create(game.into_inner(), &conn)?;

    Ok(HttpResponse::Ok().json(game))
}

#[post("/{id}")]
pub async fn update_game(_game: Json<Game>, _pool: Data<db::Pool>) -> server::Response {
    unimplemented!();
}

#[get("/{id}")]
pub async fn get_game(game_id: Path<i64>, pool: Data<db::Pool>) -> server::Response {
    let conn = pool.get()?;

    let game: Option<Game> = web::block(move || Game::find_by_id(*game_id, &conn))
        .await
        .map_err(|e| {
            error!("{}", e);
            ServiceError::InternalServerError
        })?;

    match game {
        Some(game) => return Ok(HttpResponse::Ok().json(game)),
        None => Ok(HttpResponse::NotFound().json(format!("game not found"))),
    }
}
