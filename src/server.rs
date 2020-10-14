use std::ops::Deref;
use std::sync::mpsc;
use std::sync::Arc;

use actix::prelude::*;
use actix_cors::Cors;
use actix_identity::{CookieIdentityPolicy, IdentityService};
use actix_web::cookie::SameSite;
use actix_web::{dev, get, http, middleware, web, App, HttpResponse, HttpServer};
use actix_web_opentelemetry::{RequestMetrics, RequestTracing};
use opentelemetry::{api::KeyValue, global, sdk};
use time::Duration;

use crate::auth;
use crate::db;
use crate::ddg;
use crate::errors::ServiceError;
use crate::games;
use crate::invitations;
use crate::stats;
use crate::transactions;
use crate::users;
use crate::websocket;
use crate::websocket::server::{Sale, TransactionServer};

pub type Response = Result<HttpResponse, ServiceError>;

#[get("/health")]
async fn health() -> &'static str {
    "ok"
}

pub async fn launch(db_pool: db::Pool, session_private_key: String) -> std::io::Result<()> {
    let stats = web::Data::new(stats::Stats::new());

    let meter = sdk::Meter::new("rustfuif_api");
    let request_metrics = RequestMetrics::new(
        meter,
        Some(|req: &dev::ServiceRequest| {
            req.path() == "/metrics" && req.method() == http::Method::GET
        }),
    );

    // used to notify the clients when a purchase is made in your game
    let (transmitter, receiver) = mpsc::channel::<Sale>();

    let transaction_server = Arc::new(TransactionServer::default().start());

    TransactionServer::listener(transaction_server.clone(), receiver);

    HttpServer::new(move || {
        App::new()
            .data(db_pool.clone())
            .data(transmitter.clone())
            .data(transaction_server.deref().clone())
            .app_data(stats.clone())
            .wrap(middleware::DefaultHeaders::new().header("X-Version", env!("CARGO_PKG_VERSION")))
            .wrap(middleware::Compress::default())
            .wrap(middleware::Logger::default())
            .wrap(middleware::NormalizePath::default())
            .wrap(stats::Middleware::default())
            .wrap(request_metrics.clone())
            .wrap(RequestTracing::default())
            .wrap(Cors::new().supports_credentials().finish())
            .wrap(IdentityService::new(
                CookieIdentityPolicy::new(&session_private_key.as_bytes())
                    .name("auth-cookie")
                    .same_site(SameSite::Strict)
                    .visit_deadline(Duration::weeks(2))
                    .max_age_time(Duration::weeks(2))
                    .secure(false),
            ))
            .data(web::JsonConfig::default().limit(262_144))
            .data(web::PayloadConfig::default().limit(262_144))
            .service(stats::route)
            .service(web::resource("/ws/{game_id}").to(websocket::transactions::route))
            .service(
                web::scope("/api")
                    .configure(games::routes::register)
                    .configure(invitations::routes::register)
                    .configure(auth::routes::register)
                    .configure(transactions::routes::register)
                    .configure(users::routes::register)
                    .configure(ddg::routes::register)
                    .service(health),
            )
            .service(web::scope("/admin").service(health))
    })
    .bind(format!(
        "{}:{}",
        std::env::var("API_HOST").unwrap_or_else(|_| "localhost".to_string()),
        std::env::var("API_PORT").unwrap_or_else(|_| "8080".to_string())
    ))?
    .run()
    .await
}

pub fn init_tracer(agent_endpoint: &str) -> std::io::Result<()> {
    let exporter: opentelemetry_jaeger::Exporter = opentelemetry_jaeger::Exporter::builder()
        .with_agent_endpoint(
            agent_endpoint
                .parse()
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?,
        )
        .with_process(opentelemetry_jaeger::Process {
            service_name: "rustfuif".to_string(),
            tags: Vec::new(),
        })
        .init()
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
    let provider = sdk::Provider::builder()
        .with_simple_exporter(exporter)
        .with_config(sdk::Config {
            default_sampler: Box::new(sdk::Sampler::AlwaysOn),
            resource: Arc::new(sdk::Resource::new(vec![
                KeyValue::new("service.name", "rustfuif-api"),
                KeyValue::new("service.namespace", "rustfuif"),
                KeyValue::new("service.version", env!("CARGO_PKG_VERSION")),
            ])),
            ..Default::default()
        })
        .build();
    global::set_provider(provider);

    Ok(())
}
