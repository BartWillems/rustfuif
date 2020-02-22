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
extern crate actix_threadpool;
extern crate actix_web;
extern crate chrono;
extern crate derive_more;
extern crate dotenv;
extern crate terminator;

use dotenv::dotenv;
use terminator::Terminator;

mod db;
mod errors;
mod games;
mod schema;
mod server;

#[actix_rt::main]
async fn main() -> Result<(), Terminator> {
    dotenv().ok();

    env_logger::init();

    let database_url = std::env::var("DATABASE_URL").or(Err("DATABASE_URL must be set"))?;

    debug!("building database connection pool");
    let pool = db::build_connection_pool(&database_url)?;

    debug!("running database migrations");
    db::migrate(&pool)?;

    debug!("launching the actix webserver");
    server::launch(pool).await?;

    Ok(())
}
