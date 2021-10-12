use actix_identity::Identity;
use actix_web::web::{Data, Json};
use actix_web::{get, post, web};

use crate::auth;
use crate::config::Config;
use crate::games::Game;
use crate::market::MarketAgent;
use crate::server::{Response, State};
use crate::users::User;
use crate::websocket::queries::{ActiveGames, ConnectedUsers};

#[get("/admin/games/count")]
async fn game_count(state: Data<State>, id: Identity) -> Response {
    auth::verify_admin(&id)?;

    let count = Game::count(&state.db).await?;

    http_ok_json!(count);
}

#[get("/admin/users/count")]
async fn user_count(state: Data<State>, id: Identity) -> Response {
    auth::verify_admin(&id)?;

    let count = User::count(&state.db).await?;

    http_ok_json!(count);
}

#[get("/admin/websockets/connected-users")]
async fn connected_users(id: Identity, state: Data<State>) -> Response {
    auth::verify_admin(&id)?;

    let res = state.notifier.send(ConnectedUsers).await?;

    match res {
        Ok(users) => http_ok_json!(users),
        Err(err) => {
            error!("unable to fetch the users: {}", err);
            Err(crate::errors::ServiceError::InternalServerError)
        }
    }
}

#[get("/admin/websockets/active-games")]
async fn active_games(id: Identity, state: Data<State>) -> Response {
    auth::verify_admin(&id)?;

    let res = state.notifier.send(ActiveGames).await?;

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

#[get("/admin/server/stats")]
async fn server_stats(id: Identity) -> Response {
    auth::verify_admin(&id)?;

    #[derive(Serialize)]
    struct Stats {
        requests: usize,
        errors: usize,
        cache_hits: usize,
        cache_misses: usize,
    }

    http_ok_json!(Stats {
        requests: crate::stats::Stats::load_requests(),
        errors: crate::stats::Stats::load_errors(),
        cache_hits: crate::cache::Stats::load_hits(),
        cache_misses: crate::cache::Stats::load_misses(),
    });
}

#[get("/admin/server/database")]
async fn database_stats(id: Identity, state: Data<State>) -> Response {
    auth::verify_admin(&id)?;

    #[derive(Serialize)]
    struct Stats {
        active_db_connections: usize,
        idle_db_connections: usize,
    }

    http_ok_json!(Stats {
        active_db_connections: state.db.size() as usize,
        idle_db_connections: state.db.num_idle(),
    })
}

#[post("/admin/market/update-prices")]
async fn update_prices(id: Identity) -> Response {
    auth::verify_admin(&id)?;

    // TODO: Use the select macro to send things to the agents?
    // state.market.update().await;

    bad_request!("not supported for now");

    // http_ok_json!("Prices have been updates succesfully");
}

#[get("/admin/market/update-interval")]
async fn get_price_update_interval(id: Identity) -> Response {
    auth::verify_admin(&id)?;

    http_ok_json!(MarketAgent::interval().as_secs());
}

#[post("/admin/market/update-interval")]
async fn set_price_update_interval(id: Identity, seconds: Json<u64>) -> Response {
    auth::verify_admin(&id)?;

    Config::set_price_update_interval(*seconds);

    http_ok_json!(MarketAgent::interval().as_secs());
}

pub fn register(cfg: &mut web::ServiceConfig) {
    cfg.service(game_count);
    cfg.service(user_count);
    cfg.service(connected_users);
    cfg.service(active_games);
    cfg.service(cache_status);
    cfg.service(disable_cache);
    cfg.service(enable_cache);
    cfg.service(server_stats);
    cfg.service(database_stats);
    cfg.service(update_prices);
    cfg.service(get_price_update_interval);
    cfg.service(set_price_update_interval);
}
