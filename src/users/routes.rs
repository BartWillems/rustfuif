use actix_identity::Identity;
use actix_web::get;
use actix_web::web;
use actix_web::web::{Data, Query};

use crate::auth;
use crate::db;
use crate::server;
use crate::users::{Filter, User};

#[get("/users")]
async fn find_all(query: Query<Filter>, pool: Data<db::Pool>, id: Identity) -> server::Response {
    auth::get_user(&id)?;

    let users = web::block(move || User::find_all(query.into_inner(), &pool.get()?)).await?;

    http_ok_json!(users);
}

pub fn register(cfg: &mut web::ServiceConfig) {
    cfg.service(find_all);
}
