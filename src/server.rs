use std::ops::Deref;
use std::sync::mpsc;
use std::sync::Arc;

use actix::prelude::*;
use actix_cors::Cors;
use actix_files as fs;
use actix_identity::{CookieIdentityPolicy, IdentityService};
use actix_web::{get, middleware, web, App, HttpResponse, HttpServer};

use crate::auth;
use crate::db;
use crate::errors::ServiceError;
use crate::games;
use crate::invitations;
use crate::metrics;
use crate::transactions;
use crate::users;
use crate::websocket;
use crate::websocket::server::{Sale, TransactionServer};

pub type Response = Result<HttpResponse, ServiceError>;

#[get("/health")]
async fn health() -> &'static str {
    "ok"
}

pub async fn launch(db_pool: db::Pool, session_private_key: String) -> std::io::Result<()> {
    let metrics = web::Data::new(metrics::Metrics::new());

    // used to notify the clients when a purchase is made in your game
    let (transmitter, receiver) = mpsc::channel::<Sale>();

    let transaction_server = Arc::new(TransactionServer::default().start());

    TransactionServer::listener(transaction_server.clone(), receiver);

    HttpServer::new(move || {
        App::new()
            .data(db_pool.clone())
            .data(transmitter.clone())
            .data(transaction_server.deref().clone())
            .app_data(metrics.clone())
            .wrap(middleware::DefaultHeaders::new().header("X-Version", env!("CARGO_PKG_VERSION")))
            .wrap(middleware::Compress::default())
            .wrap(middleware::Logger::default())
            .wrap(middleware::NormalizePath)
            .wrap(metrics::Middleware::default())
            .wrap(Cors::new().supports_credentials().finish())
            .wrap(IdentityService::new(
                CookieIdentityPolicy::new(&session_private_key.as_bytes())
                    .name("auth-cookie")
                    .secure(false),
            ))
            .data(web::JsonConfig::default().limit(262_144))
            .data(web::PayloadConfig::default().limit(262_144))
            .service(metrics::route)
            .service(web::resource("/ws/{game_id}").to(websocket::transactions::route))
            .service(
                web::scope("/api")
                    .configure(games::routes::register)
                    .configure(invitations::routes::register)
                    .configure(auth::routes::register)
                    .configure(transactions::routes::register)
                    .configure(users::routes::register)
                    .service(health)
                    .service(fs::Files::new("/spec", "./api-spec").index_file("index.html")),
            )
            .service(web::scope("/admin").service(health))
    })
    .bind(format!(
        "{}:{}",
        std::env::var("API_HOST").unwrap_or_else(|_| "localhost".to_string()),
        std::env::var("API_PORT").unwrap_or_else(|_| "8080".to_string())
    ))?
    .run()
    .await
}
