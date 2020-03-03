use actix_session::Session;
use actix_web::http::StatusCode;
use actix_web::web;
use actix_web::web::{Data, HttpResponse, Json, Path, Query};
use actix_web::{delete, get, post, put};

use crate::auth;
use crate::db;
use crate::errors::ServiceError;
use crate::server;

use crate::games::game::{CreateGame, Game, GameQuery, UserInvite};
use crate::invitations::State;

#[get("/games")]
async fn find_all(query: Query<GameQuery>, pool: Data<db::Pool>) -> server::Response {
    let conn = pool.get()?;

    let games: Vec<Game> = web::block(move || Game::find_all(query.into_inner(), &conn)).await?;

    http_ok_json!(games);
}

#[get("/games/{id}")]
async fn find(game_id: Path<i64>, pool: Data<db::Pool>) -> server::Response {
    let conn = pool.get()?;

    let game = web::block(move || Game::find_by_id(*game_id, &conn)).await?;

    http_ok_json!(game);
}

/// get users who partake in a game, aka, invited users that have accepted
/// TODO: figure out if this should become /game/$id/users/invitations?state=accepted
#[get("/games/{id}/users")]
async fn find_users(
    game_id: Path<i64>,
    query: Query<Option<State>>,
    pool: Data<db::Pool>,
) -> server::Response {
    let conn = pool.get()?;

    let users = web::block(move || Game::find_users(*game_id, query.into_inner(), &conn)).await?;

    http_ok_json!(users);
}

/// Invite a user to a game
#[post("/games/{id}/users/invitations")]
async fn invite_user(
    game_id: Path<i64>,
    invite: Json<UserInvite>,
    session: Session,
    pool: Data<db::Pool>,
) -> server::Response {
    let conn = pool.get()?;
    let owner_id = auth::get_user_id(&session)?;

    let invite = invite.into_inner();

    web::block(move || {
        let game = Game::find_by_id(*game_id, &conn)?;
        if game.owner_id != owner_id {
            forbidden!("Only the game owner can invite users");
        }

        game.invite_user(invite.user_id, &conn)
    })
    .await?;

    Ok(HttpResponse::new(StatusCode::CREATED))
}

#[post("/games")]
async fn create(
    game: Json<CreateGame>,
    pool: Data<db::Pool>,
    session: Session,
) -> server::Response {
    let mut game = game.into_inner();
    game.owner_id = auth::get_user_id(&session)?;

    let conn = pool.get()?;

    let game = web::block(move || Game::create(game, &conn)).await?;

    http_created_json!(game);
}

#[put("/games")]
async fn update(game: Json<Game>, pool: Data<db::Pool>, session: Session) -> server::Response {
    auth::validate_session(&session)?;

    let conn = pool.get()?;

    let game = web::block(move || game.update(&conn)).await?;

    http_ok_json!(game);
}

#[delete("/games/{id}")]
async fn delete(game_id: Path<i64>, pool: Data<db::Pool>, session: Session) -> server::Response {
    let user_id = auth::get_user_id(&session)?;
    let is_admin = auth::is_admin(&session)?;

    let conn = pool.get()?;

    web::block(move || {
        let game = Game::find_by_id(*game_id, &conn)?;
        if game.owner_id != user_id && !is_admin {
            forbidden!("Only game owners can delete games");
        }
        Game::delete_by_id(game.id, &conn)
    })
    .await?;

    Ok(HttpResponse::new(StatusCode::OK))
}

pub fn register(cfg: &mut web::ServiceConfig) {
    cfg.service(find_all);
    cfg.service(find);
    cfg.service(create);
    cfg.service(update);
    cfg.service(delete);

    cfg.service(invite_user);
    cfg.service(find_users);
}
