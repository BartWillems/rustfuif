use actix_identity::Identity;
use actix_web::get;
use actix_web::web;
use actix_web::web::{Data, Query};

use crate::auth;
use crate::server::{Response, State};
use crate::users::{Filter, User};

#[get("/users")]
async fn find_all(filter: Query<Filter>, state: Data<State>, id: Identity) -> Response {
    auth::get_user(&id)?;

    let users = User::find_all(filter.into_inner(), &state.db).await?;

    http_ok_json!(users);
}

#[get("/users/me")]
async fn find_me(state: Data<State>, id: Identity) -> Response {
    let user = auth::get_user(&id)?;

    let user = User::find(user.id, &state.db).await?;

    http_ok_json!(user);
}

pub fn register(cfg: &mut web::ServiceConfig) {
    cfg.service(find_all);
    cfg.service(find_me);
}
