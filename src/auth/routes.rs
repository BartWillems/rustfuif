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

    let is_valid = user.verify_password(password.as_bytes())?;

    if !is_valid {
        return Err(ServiceError::Unauthorized);
    }

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

pub fn register(cfg: &mut web::ServiceConfig) {
    cfg.service(create_account);
    cfg.service(login);
    cfg.service(logout);
}
