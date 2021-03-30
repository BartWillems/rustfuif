use crate::auth;
use crate::errors::ServiceError;
use crate::server::{Response, State};
use crate::users::{Credentials, User};
use crate::validator::Validator;

use actix_identity::Identity;
use actix_web::web::{Data, Json};
use actix_web::{get, post, web, HttpResponse};
use serde_json::json;

#[post("/register")]
async fn create_account(credentials: Json<Validator<Credentials>>, state: Data<State>) -> Response {
    let user = User::create(&mut credentials.into_inner().validate()?, &state.db).await?;

    http_created_json!(user);
}

#[post("/login")]
async fn login(credentials: Json<Credentials>, id: Identity, state: Data<State>) -> Response {
    let user = User::find_by_name(&credentials.username, &state.db)
        .await
        .map_err(|error| match error {
            sqlx::Error::RowNotFound => ServiceError::Unauthorized,
            _ => error.into(),
        })?;

    user.verify_password(&credentials.password.as_bytes())?;

    let user_string = serde_json::to_string(&user).map_err(|e| {
        error!("unable to serialize the user struct: {}", e);
        ServiceError::InternalServerError
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
    state: Data<State>,
    id: Identity,
) -> Response {
    let session = auth::get_user(&id)?;

    let password_change = password_change.into_inner().validate()?;

    let mut user = User::find(session.id, &state.db).await?;
    user.verify_password(password_change.old.as_bytes())?;
    user.update_password(password_change.new, &state.db).await?;

    http_ok_json!("password succesfully updated")
}

#[get("/verify-session")]
async fn verify_session(id: Identity) -> Response {
    auth::get_user(&id)?;

    http_ok_json!("session is valid")
}

pub fn register(cfg: &mut web::ServiceConfig) {
    cfg.service(create_account);
    cfg.service(login);
    cfg.service(logout);
    cfg.service(change_password);
    cfg.service(verify_session);
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_identity::Identity;
    use actix_identity::{CookieIdentityPolicy, IdentityService};
    use actix_web::http::StatusCode;
    use actix_web::test::{self, TestRequest};
    use actix_web::{web, App, HttpResponse};

    const COOKIE_KEY_MASTER: [u8; 32] = [0; 32];
    const COOKIE_NAME: &str = "actix_auth";

    #[actix_rt::test]
    async fn test_identity() {
        let mut srv = test::init_service(
            App::new()
                .wrap(IdentityService::new(
                    CookieIdentityPolicy::new(&COOKIE_KEY_MASTER)
                        .domain("localhost")
                        .name(COOKIE_NAME)
                        .path("/")
                        .secure(true),
                ))
                .service(verify_session)
                .service(web::resource("/login").to(|id: Identity| {
                    let user = User {
                        id: 1,
                        is_admin: true,
                        username: "admin".to_string(),
                        password: "admin".to_string(),
                        created_at: None,
                        updated_at: None,
                    };

                    let user_string = serde_json::to_string(&user).unwrap();

                    id.remember(user_string);
                    HttpResponse::Ok()
                })),
        )
        .await;

        let resp = test::call_service(
            &mut srv,
            TestRequest::with_uri("/verify-session").to_request(),
        )
        .await;
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

        let resp = test::call_service(&mut srv, TestRequest::with_uri("/login").to_request()).await;
        let cookie = resp.response().cookies().next().unwrap().to_owned();

        let resp = test::call_service(
            &mut srv,
            TestRequest::with_uri("/verify-session")
                .cookie(cookie.clone())
                .to_request(),
        )
        .await;
        assert_eq!(resp.status(), StatusCode::OK);
    }
}
