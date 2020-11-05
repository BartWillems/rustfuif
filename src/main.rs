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

mod auth;
mod cache;
mod db;
mod ddg;
mod errors;
mod games;
mod invitations;
mod prices;
mod schema;
mod server;
mod stats;
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

    // TODO: use a configuration helper
    let session_private_key = get_env("SESSION_PRIVATE_KEY")?;
    if session_private_key.len() < 32 {
        return Err(Box::from(format!(
            "session private key should be at least 32 bytes, found: {}",
            session_private_key.len()
        )));
    }

    match server::init_tracer(&get_env("OPENTELEMETRY_AGENT")?) {
        Err(e) => error!("Error: {}, no jaeger traces will be sent", e),
        _ => {
            info!("jaeger tracing enabled");
        }
    }

    let database_url = get_env("DATABASE_URL")?;

    debug!("building database connection pool");
    let pool = db::build_connection_pool(&database_url)?;

    info!("running database migrations");
    db::migrate(&pool)?;

    cache::init();

    debug!("launching the actix webserver");
    server::launch(pool.clone(), session_private_key).await?;

    Ok(())
}

fn get_env(key: &str) -> Result<String, Box<dyn std::error::Error>> {
    let res = std::env::var(key).map_err(|_| format!("{} must be set", key))?;
    Ok(res)
}
