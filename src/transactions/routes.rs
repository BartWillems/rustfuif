use actix_session::Session;
use actix_web::web;
use actix_web::web::{Data, Json, Path};
use actix_web::{get, post};

use crate::auth;
use crate::db;
use crate::server;
use crate::transactions::models::{Sale, Transaction, TransactionFilter};

#[get("/games/{id}/sales")]
async fn get_sales(game_id: Path<i64>, session: Session, pool: Data<db::Pool>) -> server::Response {
    let user_id = auth::get_user_id(&session)?;
    let conn = pool.get()?;
    let game_id = game_id.into_inner();

    let filter = TransactionFilter {
        user_id: Some(user_id),
        game_id: Some(game_id),
    };

    let transactions = web::block(move || Transaction::find_all(&filter, &conn)).await?;

    http_ok_json!(transactions);
}

#[post("/games/{id}/sales")]
async fn create_sale(
    game_id: Path<i64>,
    slot_no: Json<i16>,
    session: Session,
    pool: Data<db::Pool>,
) -> server::Response {
    let user_id = auth::get_user_id(&session)?;
    let conn = pool.get()?;

    let sale = Sale {
        user_id,
        game_id: game_id.into_inner(),
        slot_no: slot_no.into_inner(),
    };

    let transaction = web::block(move || sale.save(&conn)).await?;
    http_created_json!(transaction);
}

pub fn register(cfg: &mut web::ServiceConfig) {
    cfg.service(get_sales);
    cfg.service(create_sale);
}
