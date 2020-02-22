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
        App::new()
            .data(db_pool.clone())
            .wrap(middleware::DefaultHeaders::new().header("X-Version", env!("CARGO_PKG_VERSION")))
            .wrap(middleware::Compress::default())
            .wrap(middleware::Logger::default())
            .wrap(middleware::NormalizePath)
            // limit the maximum amount of data that server will accept
            .data(web::JsonConfig::default().limit(262_144))
            .data(web::PayloadConfig::default().limit(262_144))
            .service(
                web::scope("/api")
                    .service(
                        web::scope("/games")
                            .service(games::routes::create_game)
                            .service(games::routes::get_games)
                            .service(games::routes::get_game),
                    )
                    .service(health),
            )
            .service(web::scope("/admin").service(health))
    })
    .bind("0.0.0.0:8080")?
    .run()
    .await
}
