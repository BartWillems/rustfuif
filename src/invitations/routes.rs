use actix_session::Session;
use actix_web::web::{Data, HttpResponse};
use actix_web::{get, web};

use crate::auth;
use crate::db;
use crate::invitations::Invitation;
use crate::server;

#[get("/invitations")]
async fn my_invitations(session: Session, pool: Data<db::Pool>) -> server::Response {
    let owner_id = auth::get_user_id(&session)?;
    let conn = pool.get()?;

    let invitations = web::block(move || Invitation::find(owner_id, &conn)).await?;

    http_ok_json!(invitations);
}

pub fn register(cfg: &mut web::ServiceConfig) {
    cfg.service(my_invitations);
}
