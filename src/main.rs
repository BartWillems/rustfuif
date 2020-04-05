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

#[macro_use]
mod macros;

mod auth;
mod db;
mod errors;
mod games;
mod invitations;
mod metrics;
mod schema;
mod server;
mod transactions;
mod users;
mod validator;
mod websocket;

#[actix_rt::main]
async fn main() -> Result<(), Terminator> {
    init().await?;

    Ok(())
}

async fn init() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();

    env_logger::init();

    let session_private_key = get_env("SESSION_PRIVATE_KEY")?;
    if session_private_key.len() < 32 {
        return Err(Box::from(format!(
            "session private key should be at least 32 bytes, found: {}",
            session_private_key.len()
        )));
    }

    let database_url = get_env("DATABASE_URL")?;

    debug!("building database connection pool");
    let pool = db::build_connection_pool(&database_url)?;

    debug!("running database migrations");
    db::migrate(&pool)?;

    debug!("launching the actix webserver");
    server::launch(pool, session_private_key).await?;

    Ok(())
}

fn get_env(key: &str) -> Result<String, Box<dyn std::error::Error>> {
    let res = std::env::var(key).or_else(|_| Err(format!("{} must be set", key)))?;
    Ok(res)
}
