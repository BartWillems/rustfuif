//!
#![warn(missing_debug_implementations, rust_2018_idioms, missing_docs)]

#[macro_use]
extern crate lazy_static;

#[macro_use]
extern crate log;

#[macro_use]
extern crate serde_derive;

use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::prelude::*;

use anyhow::Error;
use dotenv::dotenv;

#[macro_use]
mod macros;

mod admin;
mod auth;
mod cache;
mod config;
mod ddg;
mod errors;
mod games;
mod invitations;
mod market;
mod server;
mod stats;
mod transactions;
mod users;
mod validator;
mod websocket;

#[actix_web::main]
async fn main() -> anyhow::Result<(), Error> {
    init().await?;

    Ok(())
}

async fn init() -> anyhow::Result<(), Error> {
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

    cache::Cache::init();

    debug!("launching the actix webserver");
    server::launch().await?;

    Ok(())
}
