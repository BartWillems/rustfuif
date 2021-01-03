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

use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::prelude::*;

use dotenv::dotenv;
use terminator::Terminator;

#[macro_use]
mod macros;

mod admin;
mod auth;
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

    let (tracer, _uninstall) = opentelemetry_jaeger::new_pipeline()
        .with_service_name("rustfuif")
        .with_agent_endpoint(config::Config::opentelemetry_endpoint())
        .install()
        .expect("unable to connect to opentelemetry agent");

    // Create a tracing layer with the configured tracer
    let opentelemetry = tracing_opentelemetry::layer().with_tracer(tracer);

    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer().with_writer(std::io::stdout))
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .with(opentelemetry)
        .try_init()
        .expect("unable to initialize the tokio tracer");

    debug!("building database connection pool");
    let pool = db::build_connection_pool(config::Config::database_url())?;

    info!("running database migrations");
    db::migrate(&pool)?;

    if let Some(redis_url) = config::Config::redis_url() {
        info!("launching cache");
        match rustfuif_cache::Cache::init(redis_url.to_string()).await {
            Ok(()) => info!("cache initialized"),
            Err(e) => error!("unable to initiate cache pool: {}", e),
        };
    }

    debug!("launching the actix webserver");
    server::launch(pool.clone()).await?;

    Ok(())
}
