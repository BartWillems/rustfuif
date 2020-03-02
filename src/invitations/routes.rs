use actix_session::Session;
use actix_web::http::StatusCode;
use actix_web::web::{Data, HttpResponse, Json};
use actix_web::{get, post, web};

use crate::auth;
use crate::db;
use crate::errors::ServiceError;
use crate::games::Game;
use crate::invitations::{Invitation, InviteMessage};
use crate::server;

#[get("/invitations")]
async fn my_invitations(session: Session, pool: Data<db::Pool>) -> server::Response {
    let owner_id = auth::get_user_id(&session)?;
    let conn = pool.get()?;

    let invitations = web::block(move || Invitation::find(owner_id, &conn)).await?;

    Ok(HttpResponse::Ok().json(invitations))
}

#[post("/invitations")]
async fn invite_user(
    invite: Json<InviteMessage>,
    session: Session,
    pool: Data<db::Pool>,
) -> server::Response {
    let conn = pool.get()?;
    let owner_id = auth::get_user_id(&session)?;

    let invite = invite.into_inner();

    web::block(move || {
        let game = Game::find_by_id(invite.game_id, &conn)?;

        if game.owner_id != owner_id {
            forbidden!("Only game owners can delete games");
        }

        game.invite_user(invite.user_id, &conn)
    })
    .await?;

    Ok(HttpResponse::new(StatusCode::CREATED))
}

pub fn register(cfg: &mut web::ServiceConfig) {
    cfg.service(my_invitations);
    cfg.service(invite_user);
}
