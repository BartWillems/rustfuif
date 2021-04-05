use std::pin::Pin;
use std::sync::atomic::{AtomicU32, Ordering};
use std::task::{Context, Poll};

use actix::Addr;
use actix_service::{Service, Transform};
use actix_web::dev::{ServiceRequest, ServiceResponse};
use actix_web::web::Data;
use actix_web::Error;
use actix_web::{get, web};
use futures::future::{ok, Ready};
use futures::Future;

use crate::db;
use crate::server::Response;
use crate::websocket::queries::ActiveSessionCount;
use crate::websocket::server::NotificationServer;

lazy_static! {
    static ref STATS: Stats = Stats::new();
}

pub struct Stats {
    requests: AtomicU32,
    errors: AtomicU32,
    cache_hits: AtomicU32,
    cache_misses: AtomicU32,
}

/// This is used to expose the raw stats without needing to declare new
/// atomic variables
#[derive(Serialize)]
pub struct LoadedStats {
    requests: u32,
    errors: u32,
    cache_hits: u32,
    cache_misses: u32,
}

impl Stats {
    pub fn new() -> Stats {
        Stats {
            requests: AtomicU32::new(0),
            errors: AtomicU32::new(0),
            cache_hits: AtomicU32::new(0),
            cache_misses: AtomicU32::new(0),
        }
    }

    pub fn add_request() {
        STATS.requests.fetch_add(1, Ordering::Relaxed);
    }

    pub fn add_error() {
        STATS.errors.fetch_add(1, Ordering::Relaxed);
    }

    pub fn cache_hit() {
        STATS.cache_hits.fetch_add(1, Ordering::Relaxed);
    }

    pub fn cache_miss() {
        STATS.cache_misses.fetch_add(1, Ordering::Relaxed);
    }

    /// Load the atomic stats variables as regular u32's
    pub fn load() -> LoadedStats {
        LoadedStats {
            requests: STATS.requests.load(Ordering::Relaxed),
            errors: STATS.errors.load(Ordering::Relaxed),
            cache_hits: STATS.cache_hits.load(Ordering::Relaxed),
            cache_misses: STATS.cache_misses.load(Ordering::Relaxed),
        }
    }
}

#[derive(Serialize)]
pub struct StatsResponse {
    requests: u32,
    errors: u32,
    active_ws_sessions: usize,
    active_games: i64,
    active_db_connections: u32,
    idle_db_connections: u32,
    cache_hits: u32,
    cache_misses: u32,
}

#[get("/stats")]
pub async fn route(sessions: Data<Addr<NotificationServer>>, pool: Data<db::Pool>) -> Response {
    let state = pool.clone().into_inner().state();

    let active_games = web::block(move || {
        let conn = pool.get()?;
        crate::games::Game::active_game_count(&conn)
    })
    .await?;

    http_ok_json!(StatsResponse {
        requests: STATS.requests.load(Ordering::Relaxed),
        errors: STATS.errors.load(Ordering::Relaxed),
        active_ws_sessions: sessions.get_ref().send(ActiveSessionCount).await?,
        active_games,
        active_db_connections: state.connections,
        idle_db_connections: state.idle_connections,
        cache_hits: STATS.cache_hits.load(Ordering::Relaxed),
        cache_misses: STATS.cache_misses.load(Ordering::Relaxed),
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
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>>>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.service.poll_ready(cx)
    }

    fn call(&mut self, request: ServiceRequest) -> Self::Future {
        Stats::add_request();

        let fut = self.service.call(request);

        Box::pin(async move {
            let res = fut.await?;

            if res.response().status().is_server_error() {
                Stats::add_error();
            }

            Ok(res)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use actix_web::test::{self, TestRequest};
    use actix_web::{web, App, HttpResponse};

    #[actix_rt::test]
    async fn requests_and_errors_counter() {
        let mut srv = test::init_service(
            App::new()
                .wrap(Middleware::default())
                .service(web::resource("/success").to(|| HttpResponse::Ok()))
                .service(web::resource("/failure").to(|| HttpResponse::InternalServerError())),
        )
        .await;

        test::call_service(&mut srv, TestRequest::with_uri("/success").to_request()).await;
        assert_eq!(STATS.requests.load(Ordering::Relaxed), 1);
        assert_eq!(STATS.errors.load(Ordering::Relaxed), 0);

        test::call_service(&mut srv, TestRequest::with_uri("/failure").to_request()).await;
        assert_eq!(STATS.requests.load(Ordering::Relaxed), 2);
        assert_eq!(STATS.errors.load(Ordering::Relaxed), 1);
    }
}
