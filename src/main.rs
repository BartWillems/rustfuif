#[macro_use]
extern crate diesel;

#[macro_use]
extern crate diesel_migrations;

#[macro_use]
extern crate lazy_static;

#[macro_use]
extern crate log;

#[macro_use]
extern crate serde_derive;

use dotenv::dotenv;
use terminator::Terminator;

#[macro_use]
mod macros;

mod admin;
mod auth;
mod cache;
mod config;
mod db;
mod ddg;
mod errors;
mod games;
mod invitations;
mod market;
mod prices;
mod schema;
mod server;
mod stats;
mod transactions;
mod users;
mod validator;
mod websocket;

#[actix_web::main]
async fn main() -> Result<(), Terminator> {
    init().await?;

    Ok(())
}

async fn init() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();

    env_logger::init();

    debug!("building database connection pool");
    let pool = db::build_connection_pool(config::Config::database_url())?;

    info!("running database migrations");
    db::migrate(&pool)?;

    cache::Cache::init();

    debug!("launching the actix webserver");
    server::launch(pool.clone()).await?;

    Ok(())
}
