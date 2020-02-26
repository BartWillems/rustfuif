use actix_web::{get, middleware, web, App, HttpRequest, HttpResponse, HttpServer};
use diesel::{r2d2::ConnectionManager, PgConnection};

use crate::errors::ServiceError;
use crate::games;

pub type Response = Result<HttpResponse, ServiceError>;

#[get("/health")]
async fn health(_: HttpRequest) -> &'static str {
    "ok"
}

pub async fn launch(db_pool: r2d2::Pool<ConnectionManager<PgConnection>>) -> std::io::Result<()> {
    HttpServer::new(move || {
        let counter = crate::metrics::RequestCounter::new(0usize);
        App::new()
            .data(db_pool.clone())
            .app_data(counter)
            .wrap(middleware::DefaultHeaders::new().header("X-Version", env!("CARGO_PKG_VERSION")))
            .wrap(middleware::Compress::default())
            .wrap(middleware::Logger::default())
            .wrap(middleware::NormalizePath)
            .wrap(crate::metrics::PerfCounter)
            // .wrap
            // limit the maximum amount of data that server will accept
            .data(web::JsonConfig::default().limit(262_144))
            .data(web::PayloadConfig::default().limit(262_144))
            .service(
                web::scope("/api")
                    .configure(games::routes::register)
                    .service(health)
                    .service(crate::metrics::metrics),
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
