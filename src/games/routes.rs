use actix_session::Session;
use actix_web::http::StatusCode;
use actix_web::web;
use actix_web::web::{Data, HttpResponse, Json, Path, Query};
use actix_web::{delete, get, post, put};

use crate::auth;
use crate::db;
use crate::server;
use crate::validator::Validator;

use crate::games::models::{BeverageConfig, CreateGame, Game, GameFilter};

#[get("/games")]
async fn find_all(
    query: Query<GameFilter>,
    pool: Data<db::Pool>,
    session: Session,
) -> server::Response {
    let user_id = auth::get_user_id(&session)?;
    let is_admin = auth::is_admin(&session)?;

    let games = web::block(move || {
        if is_admin {
            debug!("user is admin, showing all games");
            return Game::find_all(query.into_inner(), &pool.get()?);
        } else {
            debug!("find by user");
            return Game::find_by_user(user_id, query.into_inner(), &pool.get()?);
        }
    })
    .await?;

    http_ok_json!(games);
}

#[get("/games/{id}")]
async fn find(game_id: Path<i64>, pool: Data<db::Pool>) -> server::Response {
    let game = web::block(move || Game::find_by_id(*game_id, &pool.get()?)).await?;

    http_ok_json!(game);
}

#[post("/games")]
async fn create(
    game: Json<Validator<CreateGame>>,
    pool: Data<db::Pool>,
    session: Session,
) -> server::Response {
    let mut game = game.into_inner().validate()?;

    game.owner_id = auth::get_user_id(&session)?;

    let game = web::block(move || Game::create(game, &pool.get()?)).await?;

    http_created_json!(game);
}

#[put("/games")]
async fn update(game: Json<Game>, pool: Data<db::Pool>, session: Session) -> server::Response {
    let user_id = auth::get_user_id(&session)?;
    let is_admin = auth::is_admin(&session)?;

    let game = web::block(move || {
        let conn = pool.get()?;
        let old_game = Game::find_by_id(game.id, &conn)?;
        if old_game.owner_id != user_id && !is_admin {
            forbidden!("Only game owners can delete games");
        }
        game.update(&conn)
    })
    .await?;

    http_ok_json!(game);
}

#[delete("/games/{id}")]
async fn delete(game_id: Path<i64>, pool: Data<db::Pool>, session: Session) -> server::Response {
    let user_id = auth::get_user_id(&session)?;
    let is_admin = auth::is_admin(&session)?;

    web::block(move || {
        let conn = pool.get()?;
        let game = Game::find_by_id(*game_id, &conn)?;
        if game.owner_id != user_id && !is_admin {
            forbidden!("Only game owners can delete games");
        }
        Game::delete_by_id(game.id, &conn)
    })
    .await?;

    Ok(HttpResponse::new(StatusCode::OK))
}

#[get("/games/{id}/beverages")]
async fn get_beverages(
    game_id: Path<i64>,
    pool: Data<db::Pool>,
    session: Session,
) -> server::Response {
    let user_id = auth::get_user_id(&session)?;

    let configs = web::block(move || {
        let conn = pool.get()?;
        BeverageConfig::find(*game_id, user_id, &conn)
    })
    .await?;

    http_ok_json!(configs);
}

#[post("/games/{id}/beverages")]
async fn create_beverage_config(
    game_id: Path<i64>,
    config: Json<Validator<BeverageConfig>>,
    pool: Data<db::Pool>,
    session: Session,
) -> server::Response {
    let user_id = auth::get_user_id(&session)?;
    let game_id = *game_id;
    let mut config = config.into_inner().validate()?;

    config.user_id = user_id;
    config.game_id = game_id;

    let config = web::block(move || {
        let conn = pool.get()?;
        if !Game::verify_user(game_id, user_id, &conn)? {
            forbidden!("you are not in this game");
        }

        config.save(&conn)
    })
    .await?;

    http_created_json!(config);
}

pub fn register(cfg: &mut web::ServiceConfig) {
    cfg.service(find_all);
    cfg.service(find);
    cfg.service(create);
    cfg.service(update);
    cfg.service(delete);

    cfg.service(create_beverage_config);
    cfg.service(get_beverages);
}
