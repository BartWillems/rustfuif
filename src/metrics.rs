use std::sync::atomic::{AtomicU32, Ordering};
use std::task::{Context, Poll};

use actix::Addr;
use actix_service::{Service, Transform};
use actix_web::dev::{ServiceRequest, ServiceResponse};
use actix_web::web::Data;
use actix_web::Error;
use actix_web::{get, web};
use futures::future::{ok, Either, Ready};

use crate::db;
use crate::server::Response;
use crate::websocket::server::{Query, TransactionServer};

/// the Metrics struct holds the request count
///
/// **GET /metrics**
///
/// exposes the amount of requests that this server has handled
///
/// Example:
///
/// ``` shell
/// curl localhost:8080/metrics
/// {
///     "requests": 9872
/// }
/// ```
pub struct Metrics {
    pub requests: AtomicU32,
}

impl Metrics {
    pub fn new() -> Metrics {
        Metrics {
            requests: AtomicU32::new(0u32),
        }
    }
}

#[derive(Serialize)]
pub struct MetricsResponse {
    pub requests: u32,
    pub active_ws_sessions: usize,
    pub active_games: i64,
    pub active_db_connections: u32,
    pub idle_db_connections: u32,
}

#[get("/metrics")]
pub async fn route(
    metrics: Data<Metrics>,
    sessions: Data<Addr<TransactionServer>>,
    pool: Data<db::Pool>,
) -> Response {
    let state = pool.clone().into_inner().state();
    let metrics = metrics.into_inner();

    let active_games = web::block(move || {
        let conn = pool.get()?;
        crate::games::Game::active_games(&conn)
    })
    .await?;

    http_ok_json!(MetricsResponse {
        requests: metrics.requests.load(Ordering::Relaxed),
        active_ws_sessions: sessions.get_ref().send(Query::ActiveSessions).await?,
        active_games,
        active_db_connections: state.connections,
        idle_db_connections: state.idle_connections,
    });
}

pub struct Middleware;

impl Middleware {
    pub fn default() -> Middleware {
        Middleware
    }
}

impl<S, B> Transform<S> for Middleware
where
    S: Service<Request = ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
{
    type Request = ServiceRequest;
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = RequestCountMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ok(RequestCountMiddleware { service })
    }
}

pub struct RequestCountMiddleware<S> {
    service: S,
}

impl<S, B> Service for RequestCountMiddleware<S>
where
    S: Service<Request = ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
{
    type Request = ServiceRequest;
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = Either<S::Future, Ready<Result<Self::Response, Self::Error>>>;

    fn poll_ready(&mut self, cx: &mut Context) -> Poll<Result<(), Self::Error>> {
        self.service.poll_ready(cx)
    }

    fn call(&mut self, req: ServiceRequest) -> Self::Future {
        let metrics: Option<Data<Metrics>> = req.app_data();

        // TODO: add counter bad requests
        if let Some(metrics) = metrics {
            metrics
                .into_inner()
                .requests
                .fetch_add(1, Ordering::Relaxed);
        }

        // TODO: figure out how to fix this
        Either::Left(self.service.call(req))
    }
}
