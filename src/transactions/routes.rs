use std::collections::HashMap;

use actix_identity::Identity;
use actix_web::web;
use actix_web::web::{Data, Json, Path};
use actix_web::{get, post};

use crate::auth;
use crate::db;
use crate::games::Game;
use crate::server;
use crate::server::State;
use crate::transactions::models::{NewSale, SalesCount, Transaction, TransactionFilter};
use crate::websocket::{Notification, Sale};

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
    slots: Json<HashMap<i16, i32>>,
    id: Identity,
    pool: Data<db::Pool>,
    state: Data<State>,
) -> server::Response {
    let user = auth::get_user(&id)?;
    let game_id = game_id.into_inner();

    let user_id = user.id;

    // let (transactions, sale) = web::block(move || {
    web::block(move || {
        let conn = pool.get()?;

        if !Game::is_open(game_id, user_id, &conn)? {
            forbidden!("game is not available for purchases");
        }
        Ok(())
    })
    .await?;

    let sale = NewSale {
        user_id: user.id,
        game_id,
        slots: slots.into_inner(),
    };

    let transactions = sale.save(&state.db).await?;

    if let Err(e) = state
        .notifier
        .send(Notification::NewSale(Sale {
            game_id,
            transactions: transactions.clone(),
        }))
        .await
    {
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

    let sales = web::block(move || {
        let conn = &pool.get()?;
        SalesCount::find_by_game(game_id, &conn).map_err(crate::errors::ServiceError::from)
    })
    .await?;

    http_ok_json!(sales);
}

#[get("/games/{id}/stats/users")]
async fn user_sales(game_id: Path<i64>, pool: Data<db::Pool>, id: Identity) -> server::Response {
    auth::get_user(&id)?;
    let game_id = game_id.into_inner();

    let sales = web::block(move || Transaction::get_sales_per_user(game_id, &pool.get()?)).await?;

    http_ok_json!(sales);
}

#[get("/games/{id}/stats/transactions")]
async fn get_transactions(
    game_id: Path<i64>,
    pool: Data<db::Pool>,
    id: Identity,
) -> server::Response {
    let user = auth::get_user(&id)?;
    let game_id = game_id.into_inner();

    let transactions = web::block(move || {
        Transaction::find_all(
            &TransactionFilter {
                user_id: Some(user.id),
                game_id: Some(game_id),
            },
            &pool.get()?,
        )
    })
    .await?;

    http_ok_json!(transactions);
}

#[get("/games/{id}/stats/income")]
async fn total_income(game_id: Path<i64>, pool: Data<db::Pool>, id: Identity) -> server::Response {
    auth::get_user(&id)?;
    let game_id = game_id.into_inner();

    let sales = web::block(move || Transaction::total_income(game_id, &pool.get()?)).await?;

    http_ok_json!(sales);
}

pub fn register(cfg: &mut web::ServiceConfig) {
    cfg.service(get_sales);
    cfg.service(create_sale);
    cfg.service(beverage_sales);
    cfg.service(user_sales);
    cfg.service(get_transactions);
    cfg.service(total_income);
}
