use crate::auth;
use crate::db;
use crate::errors::ServiceError;
use crate::server::Response;
use crate::users::{User, UserMessage};
use crate::validator::Validator;

use actix_identity::Identity;
use actix_web::web::{Data, Json};
use actix_web::{post, web, HttpResponse};
use serde_json::json;

#[post("/register")]
async fn create_account(user: Json<Validator<UserMessage>>, pool: Data<db::Pool>) -> Response {
    let user: User = web::block(move || {
        let conn = pool.get()?;
        User::create(&mut user.into_inner().validate()?, &conn)
    })
    .await?;

    http_created_json!(user);
}

#[post("/login")]
async fn login(credentials: Json<UserMessage>, id: Identity, pool: Data<db::Pool>) -> Response {
    let credentials = credentials.into_inner();

    // this can be removed once the web::block() is removed
    let username = credentials.username;
    let password = credentials.password;

    let user = web::block(move || {
        let conn = pool.get()?;
        User::find_by_username(username, &conn).map_err(|error| match error {
            ServiceError::NotFound => ServiceError::Unauthorized,
            _ => error,
        })
    })
    .await?;

    user.verify_password(password.as_bytes())?;

    let user_string = serde_json::to_string(&user).or_else(|e| {
        error!("unable to serialize the user struct: {}", e);
        Err(ServiceError::InternalServerError)
    })?;

    id.remember(user_string);

    http_ok_json!(user);
}

#[post("/logout")]
async fn logout(id: Identity) -> Response {
    id.forget();

    Ok(HttpResponse::Ok().json(json!({ "message": "Successfully signed out" })))
}

#[post("/change-password")]
async fn change_password(
    password_change: Json<Validator<auth::PasswordChange>>,
    pool: Data<db::Pool>,
    id: Identity,
) -> Response {
    let session_user = auth::get_user(&id)?;

    web::block(move || {
        let conn = pool.get()?;

        let password_change = password_change.into_inner().validate()?;

        let mut user = User::find(session_user.id, &conn)?;

        // old password matches
        user.verify_password(password_change.old.as_bytes())?;

        user.password = password_change.new;

        user.update_password(&conn)
    })
    .await?;

    http_ok_json!("password succesfully updated")
}

pub fn register(cfg: &mut web::ServiceConfig) {
    cfg.service(create_account);
    cfg.service(login);
    cfg.service(logout);
    cfg.service(change_password);
}
