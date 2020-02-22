use actix_web::web;
use actix_web::web::{Data, HttpResponse, Json, Path, Query};
use actix_web::{delete, get, patch, post};

use crate::db;
use crate::server;

use crate::games::models::{CreateGame, Game, GameQuery};

#[get("/")]
pub async fn get_games(query: Query<GameQuery>, pool: Data<db::Pool>) -> server::Response {
    let conn = pool.get()?;

    let games: Vec<Game> = web::block(move || {
        if query.active.unwrap_or(false) {
            Game::load_active(&conn)
        } else {
            Game::load(&conn)
        }
    })
    .await?;

    Ok(HttpResponse::Ok().json(games))
}

#[post("/")]
pub async fn create_game(game: Json<CreateGame>, pool: Data<db::Pool>) -> server::Response {
    let conn = pool.get()?;

    let game = web::block(move || Game::create(game.into_inner(), &conn)).await?;

    Ok(HttpResponse::Ok().json(game))
}

#[patch("/{id}")]
pub async fn update_game(_game: Json<Game>, _pool: Data<db::Pool>) -> server::Response {
    unimplemented!();
}

#[get("/{id}")]
pub async fn get_game(game_id: Path<i64>, pool: Data<db::Pool>) -> server::Response {
    let conn = pool.get()?;

    let game: Option<Game> = web::block(move || Game::find_by_id(*game_id, &conn)).await?;

    match game {
        Some(game) => Ok(HttpResponse::Ok().json(game)),
        None => Ok(HttpResponse::NotFound().json("game not found".to_string())),
    }
}

#[delete("/{id}")]
pub async fn delete_game(_game: Json<Game>, _pool: Data<db::Pool>) -> server::Response {
    unimplemented!();
}
