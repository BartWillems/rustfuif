use std::ops::Deref;
use std::sync::Arc;

use actix::prelude::*;
use actix_cors::Cors;
use actix_identity::{CookieIdentityPolicy, IdentityService};
use actix_web::cookie::SameSite;
use actix_web::error::JsonPayloadError;
use actix_web::middleware::normalize::TrailingSlash;
use actix_web::{dev, get, http, middleware, web, App, HttpRequest, HttpResponse, HttpServer};
use actix_web_opentelemetry::{RequestMetrics, RequestTracing};
use opentelemetry::{api::KeyValue, global, sdk};
use time::Duration;

use crate::admin;
use crate::auth;
use crate::db;
use crate::ddg;
use crate::errors::ServiceError;
use crate::games;
use crate::invitations;
use crate::prices;
use crate::stats;
use crate::transactions;
use crate::users;
use crate::websocket;
use crate::websocket::server::NotificationServer;

pub type Response = Result<HttpResponse, ServiceError>;

#[get("/health")]
async fn health() -> &'static str {
    "ok"
}

fn json_error_handler(error: JsonPayloadError, _: &HttpRequest) -> actix_web::Error {
    match error {
        JsonPayloadError::Overflow => ServiceError::PayloadTooLarge.into(),
        _ => ServiceError::BadRequest(error.to_string()).into(),
    }
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

    let transaction_server = Arc::new(NotificationServer::new().start());

    debug!("launching price updater");
    prices::Updater::new(
        db_pool.clone(),
        std::time::Duration::from_secs(120),
        transaction_server.clone(),
    )
    .start();

    HttpServer::new(move || {
        App::new()
            .data(db_pool.clone())
            .data(transaction_server.deref().clone())
            .app_data(stats.clone())
            .wrap(middleware::DefaultHeaders::new().header("X-Version", env!("CARGO_PKG_VERSION")))
            .wrap(middleware::Compress::default())
            .wrap(middleware::Logger::default())
            .wrap(middleware::NormalizePath::new(TrailingSlash::Trim))
            .wrap(stats::Middleware::default())
            .wrap(request_metrics.clone())
            .wrap(RequestTracing::default())
            // TODO: set this to something more restrictive
            .wrap(Cors::permissive().supports_credentials())
            .wrap(IdentityService::new(
                CookieIdentityPolicy::new(&session_private_key.as_bytes())
                    .name("auth-cookie")
                    .same_site(SameSite::Strict)
                    .visit_deadline(Duration::weeks(2))
                    .max_age_time(Duration::weeks(2))
                    // TODO: set this to true in production
                    .secure(false),
            ))
            .data(
                web::JsonConfig::default()
                    .error_handler(json_error_handler)
                    .limit(262_144),
            )
            .data(web::PayloadConfig::default().limit(262_144))
            .service(stats::route)
            .service(web::resource("/ws/{game_id}").to(websocket::routes::route))
            .service(
                web::scope("/api")
                    .configure(games::routes::register)
                    .configure(invitations::routes::register)
                    .configure(auth::routes::register)
                    .configure(transactions::routes::register)
                    .configure(users::routes::register)
                    .configure(ddg::routes::register)
                    .configure(admin::routes::register)
                    .service(health),
            )
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
