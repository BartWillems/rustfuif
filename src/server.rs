use actix_files as fs;
use actix_redis::RedisSession;
use actix_web::{get, middleware, web, App, HttpRequest, HttpResponse, HttpServer};

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

pub async fn launch(db_pool: db::Pool, redis_uri: String) -> std::io::Result<()> {
    let metrics = web::Data::new(metrics::Metrics::new());

    HttpServer::new(move || {
        App::new()
            .data(db_pool.clone())
            .app_data(metrics.clone())
            .wrap(middleware::DefaultHeaders::new().header("X-Version", env!("CARGO_PKG_VERSION")))
            .wrap(middleware::Compress::default())
            .wrap(middleware::Logger::default())
            .wrap(middleware::NormalizePath)
            .wrap(metrics::Middleware::default())
            .wrap(RedisSession::new(redis_uri.clone(), &[0; 32]))
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
        std::env::var("HOST").unwrap_or_else(|_| "localhost".to_string()),
        std::env::var("PORT").unwrap_or_else(|_| "8080".to_string())
    ))?
    .run()
    .await
}
