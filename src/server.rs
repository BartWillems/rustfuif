use std::sync::mpsc;
use std::thread;
use url::Url;

use actix_files as fs;
use actix_redis::RedisSession;
use actix_web::{cookie, get, middleware, web, App, HttpRequest, HttpResponse, HttpServer};

use crate::auth;
use crate::db;
use crate::errors::ServiceError;
use crate::games;
use crate::invitations;
use crate::metrics;
use crate::transactions;

pub type Response = Result<HttpResponse, ServiceError>;

#[get("/health")]
async fn health(_: HttpRequest) -> &'static str {
    "ok"
}

pub async fn launch(
    db_pool: db::Pool,
    redis_url: Url,
    session_private_key: String,
) -> std::io::Result<()> {
    let metrics = web::Data::new(metrics::Metrics::new());

    let (tx, rx) = mpsc::channel::<Vec<transactions::Transaction>>();

    // TODO: move this over to the websockets module
    // TODO 2 ELECTRIC BOOGALOO: make the websockets module
    thread::spawn(move || {
        for received in rx {
            debug!("received {:?}!", received);
        }
    });

    HttpServer::new(move || {
        App::new()
            .data(db_pool.clone())
            .data(tx.clone())
            .app_data(metrics.clone())
            .wrap(middleware::DefaultHeaders::new().header("X-Version", env!("CARGO_PKG_VERSION")))
            .wrap(middleware::Compress::default())
            .wrap(middleware::Logger::default())
            .wrap(middleware::NormalizePath)
            .wrap(metrics::Middleware::default())
            .wrap(
                RedisSession::new(
                    format!(
                        "{}:{}",
                        redis_url.host_str().expect("missing redis host"),
                        redis_url.port().unwrap_or(6379),
                    ),
                    &session_private_key.as_bytes(),
                )
                .cookie_same_site(cookie::SameSite::Strict),
            )
            .data(web::JsonConfig::default().limit(262_144))
            .data(web::PayloadConfig::default().limit(262_144))
            .service(metrics::route)
            .service(
                web::scope("/api")
                    .configure(games::routes::register)
                    .configure(invitations::routes::register)
                    .configure(auth::routes::register)
                    .configure(transactions::routes::register)
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
