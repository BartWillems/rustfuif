use crate::db;
use crate::errors::ServiceError;
use crate::server::Response;
use crate::users::{User, UserMessage};

use actix_session::Session;
use actix_web::http::StatusCode;
use actix_web::web::{Data, Json};
use actix_web::{post, web, HttpResponse};
use serde_json::json;

#[post("/register")]
async fn create_account(user: Json<UserMessage>, pool: Data<db::Pool>) -> Response {
    web::block(move || {
        let conn = pool.get()?;
        User::create(&mut user.into_inner(), &conn)
    })
    .await?;

    Ok(HttpResponse::new(StatusCode::OK))
}

#[post("/login")]
async fn login(credentials: Json<UserMessage>, session: Session, pool: Data<db::Pool>) -> Response {
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

    if is_valid {
        session.set("user_id", user.id)?;
        session.set("is_admin", user.is_admin)?;
        session.renew();
    } else {
        return Err(ServiceError::Unauthorized);
    }
    http_created_json!(user);
}

#[post("/logout")]
async fn logout(session: Session) -> Response {
    let id: Option<i64> = session.get("user_id")?;

    if id.is_some() {
        session.purge();
        Ok(HttpResponse::Ok().json(json!({ "message": "Successfully signed out" })))
    } else {
        Err(ServiceError::Unauthorized)
    }
}

pub fn register(cfg: &mut web::ServiceConfig) {
    cfg.service(create_account);
    cfg.service(login);
    cfg.service(logout);
}
