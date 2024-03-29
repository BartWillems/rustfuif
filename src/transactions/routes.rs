use std::collections::HashMap;

use actix_identity::Identity;
use actix_web::web;
use actix_web::web::{Data, Json, Path};
use actix_web::{get, post};

use crate::auth;
use crate::games::Game;
use crate::server;
use crate::server::State;
use crate::transactions::models::{NewSale, SalesCount, Transaction};
use crate::websocket::{server::GameId, Notification, Sale};

/// Get the total amount of sold beverages
#[get("/games/{id}/sales/beverages")]
async fn get_sales(game_id: Path<i64>, id: Identity, state: Data<State>) -> server::Response {
    let user = auth::get_user(&id)?;
    let game_id = game_id.into_inner();

    let sales = Transaction::orders(user.id, game_id, &state.db).await?;

    http_ok_json!(sales);
}

/// Get the total amount of purchases/orders
/// 1 order can contain multiple beverages
#[get("/games/{id}/sales/orders")]
async fn get_order_beverages(
    path: Path<i64>,
    id: Identity,
    state: Data<State>,
) -> server::Response {
    let user = auth::get_user(&id)?;
    let game_id = path.into_inner();
    let sales = Transaction::orders(user.id, game_id, &state.db).await?;
    http_ok_json!(sales);
}

#[post("/games/{id}/sales")]
async fn create_sale(
    game_id: Path<i64>,
    slots: Json<HashMap<i16, i32>>,
    id: Identity,
    state: Data<State>,
) -> server::Response {
    let user = auth::get_user(&id)?;
    let game_id = game_id.into_inner();

    let user_id = user.id;

    if !Game::available_for_purchases(game_id, user_id, &state.db).await? {
        forbidden!("game is not available for purchases");
    }

    let sale = NewSale {
        user_id: user.id,
        game_id,
        slots: slots.into_inner(),
    };

    let transactions = sale.save(&state.db).await?;

    if let Err(e) = state
        .notifier
        .send(Notification::NewSale(Sale {
            game_id: GameId(game_id),
            transactions: transactions.clone(),
        }))
        .await
    {
        error!("unable to notify users about transaction: {}", e);
    }

    http_created_json!(transactions);
}

#[get("/games/{id}/stats/sales")]
async fn beverage_sales(game_id: Path<i64>, state: Data<State>, id: Identity) -> server::Response {
    auth::get_user(&id)?;
    let game_id = game_id.into_inner();

    let sales = SalesCount::find_by_game(game_id, &state.db).await?;

    http_ok_json!(sales);
}

#[get("/games/{id}/stats/users")]
async fn user_sales(game_id: Path<i64>, state: Data<State>, id: Identity) -> server::Response {
    auth::get_user(&id)?;
    let game_id = game_id.into_inner();

    let sales = Transaction::get_sales_per_user(game_id, &state.db).await?;

    http_ok_json!(sales);
}

pub fn register(cfg: &mut web::ServiceConfig) {
    cfg.service(get_sales);
    cfg.service(get_order_beverages);
    cfg.service(create_sale);
    cfg.service(beverage_sales);
    cfg.service(user_sales);
}
