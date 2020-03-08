use std::sync::mpsc;

use actix_session::Session;
use actix_web::web;
use actix_web::web::{Data, Json, Path};
use actix_web::{get, post};

use crate::auth;
use crate::db;
use crate::games::Game;
use crate::server;
use crate::transactions::models::{NewSale, Slot, Transaction, TransactionFilter};

#[get("/games/{id}/sales")]
async fn get_sales(game_id: Path<i64>, session: Session, pool: Data<db::Pool>) -> server::Response {
    let user_id = auth::get_user_id(&session)?;
    let game_id = game_id.into_inner();

    let filter = TransactionFilter {
        user_id: Some(user_id),
        game_id: Some(game_id),
    };

    let transactions = web::block(move || Transaction::find_all(&filter, &pool.get()?)).await?;

    http_ok_json!(transactions);
}

#[post("/games/{id}/sales")]
async fn create_sale(
    game_id: Path<i64>,
    slots: Json<Vec<Slot>>,
    session: Session,
    pool: Data<db::Pool>,
    tx: Data<mpsc::Sender<Transaction>>,
) -> server::Response {
    let user_id = auth::get_user_id(&session)?;

    let transaction = web::block(move || {
        let conn = pool.get()?;
        let sale = NewSale {
            user_id,
            game_id: game_id.into_inner(),
            slots: slots.into_inner(),
        };
        if !Game::verify_user(sale.game_id, sale.user_id, &conn)? {
            forbidden!("you are not partaking in this game");
        }
        sale.save(&conn)
    })
    .await?;

    if let Err(e) = tx.into_inner().send(transaction.clone()) {
        error!(
            "unable to notify users about transaction(id: {}): {}",
            transaction.id, e
        );
    }

    http_created_json!(transaction);
}

pub fn register(cfg: &mut web::ServiceConfig) {
    cfg.service(get_sales);
    cfg.service(create_sale);
}
