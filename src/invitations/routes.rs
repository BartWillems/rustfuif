use actix_session::Session;
use actix_web::http::StatusCode;
use actix_web::web::{Data, HttpResponse, Json, Path, Query};
use actix_web::{get, post, web};

use crate::auth;
use crate::db;
use crate::games::Game;
use crate::invitations::{Invitation, InvitationQuery, UserInvite};
use crate::server;

#[get("/invitations")]
async fn my_invitations(session: Session, pool: Data<db::Pool>) -> server::Response {
    let owner_id = auth::get_user_id(&session)?;
    let conn = pool.get()?;

    let invitations = web::block(move || Invitation::find(owner_id, &conn)).await?;

    http_ok_json!(invitations);
}

/// show users who are invited for a specific game
#[get("/games/{id}/users")]
async fn find_users(
    game_id: Path<i64>,
    query: Query<InvitationQuery>,
    pool: Data<db::Pool>,
) -> server::Response {
    let conn = pool.get()?;

    let users = web::block(move || Game::find_users(*game_id, query.into_inner(), &conn)).await?;

    http_ok_json!(users);
}

/// Invite a user to a game
#[post("/games/{id}/users/invitations")]
async fn invite_user(
    game_id: Path<i64>,
    invite: Json<UserInvite>,
    session: Session,
    pool: Data<db::Pool>,
) -> server::Response {
    let conn = pool.get()?;
    let owner_id = auth::get_user_id(&session)?;

    let invite = invite.into_inner();

    web::block(move || {
        let game = Game::find_by_id(*game_id, &conn)?;
        if game.owner_id != owner_id {
            forbidden!("Only the game owner can invite users");
        }

        game.invite_user(invite.user_id, &conn)
    })
    .await?;

    Ok(HttpResponse::new(StatusCode::CREATED))
}

pub fn register(cfg: &mut web::ServiceConfig) {
    cfg.service(my_invitations);
    cfg.service(invite_user);
    cfg.service(find_users);
}
