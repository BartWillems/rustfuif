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

    let database_url = get_env("DATABASE_URL")?;
    let redis_host = get_env("REDIS_HOST")?;
    let redis_port = get_env("REDIS_PASSWORD")?;
    let redis_url = format!("{}:{}", redis_host, redis_port);

    debug!("building database connection pool");
    let pool = db::build_connection_pool(&database_url)?;

    debug!("running database migrations");
    db::migrate(&pool)?;

    debug!("launching the actix webserver");
    server::launch(pool, redis_url).await?;

    Ok(())
}

fn get_env(key: &str) -> Result<String, Box<dyn std::error::Error>> {
    let res = std::env::var(key).or(Err(format!("{} must be set", key)))?;
    Ok(res)
}
