use actix_identity::Identity;
use actix_web::http::StatusCode;
use actix_web::web::{Data, HttpResponse, Json, Path};
use actix_web::{get, post, web};

use crate::auth;
use crate::games::Game;
use crate::invitations::{Invitation, State, UserInvite};
use crate::server;

#[get("/invitations")]
async fn my_invitations(id: Identity, state: Data<server::State>) -> server::Response {
    let user = auth::get_user(&id)?;

    let invitations = Invitation::my_invitations(user.id, &state.db).await?;

    http_ok_json!(invitations);
}

/// show users who are invited for a specific game
#[get("/games/{id}/users")]
async fn find_users(
    game_id: Path<i64>,
    state: Data<server::State>,
    id: Identity,
) -> server::Response {
    auth::get_user(&id)?;

    let users = Game::invited_users(*game_id, &state.db).await?;

    http_ok_json!(users);
}

#[get("/games/{id}/available-users")]
async fn find_available_users(
    game_id: Path<i64>,
    state: Data<server::State>,
    id: Identity,
) -> server::Response {
    auth::get_user(&id)?;

    let users = Game::find_available_users(*game_id, &state.db).await?;

    http_ok_json!(users);
}

/// Invite a user to a game
#[post("/games/{id}/invitations")]
async fn invite_user(
    game_id: Path<i64>,
    invite: Json<UserInvite>,
    id: Identity,
    state: Data<server::State>,
) -> server::Response {
    let user = auth::get_user(&id)?;

    let invite = invite.into_inner();

    let game = Game::find_by_id(*game_id, &state.db).await?;
    if !game.is_owner(&user) {
        forbidden!("Only the game owner can invite users");
    }
    game.invite_user(invite.user_id, &state.db).await?;

    Ok(HttpResponse::new(StatusCode::CREATED))
}

#[post("/invitations/{id}/{response}")]
async fn respond(
    info: Path<(i64, State)>,
    id: Identity,
    state: Data<server::State>,
) -> server::Response {
    let user = auth::get_user(&id)?;

    let info = info.into_inner();
    let response = &info.1;

    let mut invite = Invitation::find_by_id(info.0, &state.db).await?;

    if user.id != invite.user_id && !user.is_admin {
        forbidden!("this is not the invite you're looking for");
    }

    match response {
        State::ACCEPTED => invite.accept(),
        State::DECLINED => invite.decline(),
        _ => bad_request!("you can only accept or decline an invite"),
    };

    let invite = invite.update(&state.db).await?;

    http_ok_json!(invite);
}

pub fn register(cfg: &mut web::ServiceConfig) {
    cfg.service(my_invitations);
    cfg.service(invite_user);
    cfg.service(find_users);
    cfg.service(find_available_users);
    cfg.service(respond);
}
