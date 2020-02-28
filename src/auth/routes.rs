// use crate::errors::ServiceError;
use crate::db;
use crate::errors::ServiceError;
use crate::server::Response;
use crate::users::model::{User, UserMessage};

use actix_session::Session;
use actix_web::http::StatusCode;
use actix_web::web::{Data, Json};
use actix_web::{post, web, HttpResponse};
use serde_json::json;

#[post("/register")]
async fn register(user: Json<UserMessage>, pool: Data<db::Pool>) -> Response {
    let conn = pool.get()?;

    User::create(&mut user.into_inner(), &conn)?;

    Ok(HttpResponse::new(StatusCode::OK))
}

#[post("/login")]
async fn login(credentials: Json<UserMessage>, session: Session, pool: Data<db::Pool>) -> Response {
    let conn = pool.get()?;
    let credentials = credentials.into_inner();

    let user =
        User::find_by_username(credentials.username, &conn).map_err(|error| match error {
            ServiceError::NotFound => ServiceError::Unauthorized,
            _ => error,
        })?;

    let is_valid = user.verify_password(credentials.password.as_bytes())?;

    if is_valid {
        session.set("user_id", user.id)?;
        session.renew();
    } else {
        return Err(ServiceError::Unauthorized);
    }

    Ok(HttpResponse::new(StatusCode::OK))
}

#[post("/logout")]
async fn logout(session: Session) -> Response {
    let id: Option<i64> = session.get("user_id")?;

    if let Some(_) = id {
        session.purge();
        Ok(HttpResponse::Ok().json(json!({ "message": "Successfully signed out" })))
    } else {
        Err(ServiceError::Unauthorized)
    }
}

pub fn init_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(register);
    cfg.service(login);
    cfg.service(logout);
}
