#[macro_use]
extern crate diesel;

#[macro_use]
extern crate diesel_migrations;

#[macro_use]
extern crate log;

#[macro_use]
extern crate serde_derive;

use dotenv::dotenv;
use terminator::Terminator;

mod auth;
mod db;
mod errors;
mod games;
mod metrics;
mod schema;
mod server;
mod users;

#[actix_rt::main]
async fn main() -> Result<(), Terminator> {
    dotenv().ok();

    env_logger::init();

    let database_url = std::env::var("DATABASE_URL").or(Err("DATABASE_URL must be set"))?;
    let redis_host = std::env::var("REDIS_HOST").or(Err("REDIS_HOST must be set"))?;
    let redis_port = std::env::var("REDIS_PORT").or(Err("REDIS_PORT must be set"))?;
    let redis_url = format!("{}:{}", redis_host, redis_port);

    debug!("building database connection pool");
    let pool = db::build_connection_pool(&database_url)?;

    debug!("running database migrations");
    db::migrate(&pool)?;

    debug!("launching the actix webserver");
    server::launch(pool, redis_url).await?;

    Ok(())
}
