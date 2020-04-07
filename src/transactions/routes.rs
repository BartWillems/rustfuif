use std::collections::HashMap;
use std::sync::mpsc;

use actix_identity::Identity;
use actix_web::web;
use actix_web::web::{Data, Json, Path};
use actix_web::{get, post};

use crate::auth;
use crate::db;
use crate::games::Game;
use crate::server;
use crate::transactions::models::{NewSale, Transaction, TransactionFilter};

#[get("/games/{id}/sales")]
async fn get_sales(game_id: Path<i64>, id: Identity, pool: Data<db::Pool>) -> server::Response {
    let user = auth::get_user(&id)?;
    let game_id = game_id.into_inner();

    let filter = TransactionFilter {
        user_id: Some(user.id),
        game_id: Some(game_id),
    };

    let transactions = web::block(move || Transaction::find_all(&filter, &pool.get()?)).await?;

    http_ok_json!(transactions);
}

#[post("/games/{id}/sales")]
async fn create_sale(
    game_id: Path<i64>,
    slots: Json<HashMap<i16, u8>>,
    id: Identity,
    pool: Data<db::Pool>,
    tx: Data<mpsc::Sender<i64>>,
) -> server::Response {
    let user = auth::get_user(&id)?;
    let game_id = game_id.into_inner();

    let transactions = web::block(move || {
        let conn = pool.get()?;
        let sale = NewSale {
            user_id: user.id,
            game_id,
            slots: slots.into_inner(),
        };

        if !Game::is_open(sale.game_id, sale.user_id, &conn)? {
            forbidden!("game is not available for purchases");
        }

        sale.save(&conn)
    })
    .await?;

    if let Err(e) = tx.into_inner().send(game_id) {
        error!("unable to notify users about transaction: {}", e);
    }

    http_created_json!(transactions);
}

#[get("/games/{id}/stats/sales")]
async fn beverage_sales(
    game_id: Path<i64>,
    pool: Data<db::Pool>,
    id: Identity,
) -> server::Response {
    auth::get_user(&id)?;
    let game_id = game_id.into_inner();

    let sales = web::block(move || Transaction::get_sales(game_id, &pool.get()?)).await?;

    http_created_json!(sales);
}

#[get("/games/{id}/stats/users")]
async fn user_sales(game_id: Path<i64>, pool: Data<db::Pool>, id: Identity) -> server::Response {
    auth::get_user(&id)?;
    let game_id = game_id.into_inner();

    let sales = web::block(move || Transaction::get_sales_per_user(game_id, &pool.get()?)).await?;

    http_created_json!(sales);
}

#[get("/games/{id}/stats/offsets")]
async fn beverage_sales_offsets(
    game_id: Path<i64>,
    pool: Data<db::Pool>,
    id: Identity,
) -> server::Response {
    auth::get_user(&id)?;
    let game_id = game_id.into_inner();

    let sales = web::block(move || Transaction::get_offsets(game_id, &pool.get()?)).await?;

    http_created_json!(sales);
}

pub fn register(cfg: &mut web::ServiceConfig) {
    cfg.service(get_sales);
    cfg.service(create_sale);
    cfg.service(beverage_sales);
    cfg.service(user_sales);
    cfg.service(beverage_sales_offsets);
}
