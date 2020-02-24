use actix_web::web;
use actix_web::web::{Data, HttpResponse, Json, Path, Query};
use actix_web::{delete, get, patch, post};

use crate::db;
use crate::server;

use crate::games::models::{CreateGame, Game, GameQuery};

#[get("/games")]
async fn find_all(query: Query<GameQuery>, pool: Data<db::Pool>) -> server::Response {
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

#[get("/games/{id}")]
async fn find(game_id: Path<i64>, pool: Data<db::Pool>) -> server::Response {
    let conn = pool.get()?;

    let game: Option<Game> = web::block(move || Game::find_by_id(*game_id, &conn)).await?;

    match game {
        Some(game) => Ok(HttpResponse::Ok().json(game)),
        None => Ok(HttpResponse::NotFound().json("game not found".to_string())),
    }
}

#[post("/games")]
async fn create(game: Json<CreateGame>, pool: Data<db::Pool>) -> server::Response {
    let conn = pool.get()?;

    // TODO: figure out a way to receive the DB errors.
    //       at the moment, actix_threadpool::BlockingError<E> is returned
    //       and I can't seem to figure out how to map E to DB-Errors
    let game = web::block(move || Game::create(game.into_inner(), &conn)).await?;

    Ok(HttpResponse::Created().json(game))
}

#[patch("/games/{id}")]
async fn update(_game: Json<Game>, _pool: Data<db::Pool>) -> server::Response {
    unimplemented!();
}

#[delete("/games/{id}")]
async fn delete(_game: Json<Game>, _pool: Data<db::Pool>) -> server::Response {
    unimplemented!();
}

pub fn register(cfg: &mut web::ServiceConfig) {
    cfg.service(find_all);
    cfg.service(find);
    cfg.service(create);
    cfg.service(update);
    cfg.service(delete);
}
