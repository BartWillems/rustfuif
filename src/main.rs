#[macro_use]
extern crate diesel;

#[macro_use]
extern crate diesel_migrations;

#[macro_use]
extern crate log;

#[macro_use]
extern crate serde_derive;

extern crate actix;
extern crate actix_rt;
extern crate actix_web;
extern crate chrono;
extern crate derive_more;
extern crate dotenv;
extern crate terminator;

use dotenv::dotenv;
use terminator::Terminator;

mod db;
mod errors;
mod game;
mod models;
mod schema;
mod web;

#[actix_rt::main]
async fn main() -> Result<(), Terminator> {
    init().await?;
    Ok(())
}

async fn init() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();

    env_logger::init();

    let database_url = std::env::var("DATABASE_URL").or(Err("DATABASE_URL must be set"))?;

    debug!("running database migrations");
    db::migrate(&database_url)?;

    debug!("building database connection pool");
    let pool = db::build_connection_pool(&database_url)?;

    debug!("launching the actix webserver");
    web::launch(pool).await?;

    Ok(())
}
