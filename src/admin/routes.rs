use actix::Addr;
use actix_identity::Identity;
use actix_web::web::Data;
use actix_web::{get, post, web};

use crate::auth;
use crate::db;
use crate::games::Game;
use crate::server::Response;
use crate::users::User;
use crate::websocket::queries::{ActiveGames, ConnectedUsers};
use crate::websocket::server::NotificationServer;

#[get("/admin/games/count")]
async fn game_count(pool: Data<db::Pool>, id: Identity) -> Response {
    auth::verify_admin(&id)?;

    let count = web::block(move || {
        let conn = pool.get()?;
        Game::count(&conn)
    })
    .await?;

    http_ok_json!(count);
}

#[get("/admin/users/count")]
async fn user_count(pool: Data<db::Pool>, id: Identity) -> Response {
    auth::verify_admin(&id)?;

    let count = web::block(move || {
        let conn = pool.get()?;
        User::count(&conn)
    })
    .await?;

    http_ok_json!(count);
}

#[get("/admin/websockets/connected-users")]
async fn connected_users(
    id: Identity,
    websocket_server: Data<Addr<NotificationServer>>,
) -> Response {
    auth::verify_admin(&id)?;

    let res = websocket_server.get_ref().send(ConnectedUsers).await?;

    match res {
        Ok(users) => http_ok_json!(users),
        Err(err) => {
            error!("unable to fetch the users: {}", err);
            Err(crate::errors::ServiceError::InternalServerError)
        }
    }
}

#[get("/admin/websockets/active-games")]
async fn active_games(id: Identity, websocket_server: Data<Addr<NotificationServer>>) -> Response {
    auth::verify_admin(&id)?;

    let res = websocket_server.get_ref().send(ActiveGames).await?;

    match res {
        Ok(games) => http_ok_json!(games),
        Err(err) => {
            error!("unable to fetch the active games: {}", err);
            Err(crate::errors::ServiceError::InternalServerError)
        }
    }
}

#[get("/admin/server/cache")]
async fn cache_status(id: Identity) -> Response {
    auth::verify_admin(&id)?;

    http_ok_json!(crate::cache::Cache::status().await);
}

#[post("/admin/server/cache/disable")]
async fn disable_cache(id: Identity) -> Response {
    auth::verify_admin(&id)?;

    crate::cache::Cache::disable_cache().await;

    http_ok_json!(crate::cache::Cache::status().await);
}

#[post("/admin/server/cache/enable")]
async fn enable_cache(id: Identity) -> Response {
    auth::verify_admin(&id)?;

    crate::cache::Cache::enable_cache().await;

    http_ok_json!(crate::cache::Cache::status().await);
}

pub fn register(cfg: &mut web::ServiceConfig) {
    cfg.service(game_count);
    cfg.service(user_count);
    cfg.service(connected_users);
    cfg.service(active_games);
    cfg.service(cache_status);
    cfg.service(disable_cache);
    cfg.service(enable_cache);
}
