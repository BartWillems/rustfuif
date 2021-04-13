use std::pin::Pin;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::task::{Context, Poll};

use actix_service::{Service, Transform};
use actix_web::dev::{ServiceRequest, ServiceResponse};
use actix_web::get;
use actix_web::web::Data;
use actix_web::Error;
use futures::future::{ok, Ready};
use futures::Future;

use crate::games::Game;
use crate::server::{Response, State};
use crate::websocket::queries::ActiveSessionCount;

lazy_static! {
    static ref STATS: Stats = Stats::new();
}

pub struct Stats {
    requests: AtomicUsize,
    errors: AtomicUsize,
    cache_hits: AtomicUsize,
    cache_misses: AtomicUsize,
}

/// This is used to expose the raw stats without needing to declare new
/// atomic variables
#[derive(Serialize)]
pub struct LoadedStats {
    requests: usize,
    errors: usize,
    cache_hits: usize,
    cache_misses: usize,
}

impl Stats {
    pub fn new() -> Stats {
        Stats {
            requests: AtomicUsize::new(0),
            errors: AtomicUsize::new(0),
            cache_hits: AtomicUsize::new(0),
            cache_misses: AtomicUsize::new(0),
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
    requests: usize,
    errors: usize,
    active_ws_sessions: usize,
    active_games: i64,
    active_db_connections: usize,
    idle_db_connections: usize,
    cache_hits: usize,
    cache_misses: usize,
}

#[get("/stats")]
pub async fn route(state: Data<State>) -> Response {
    let db = &state.db;

    http_ok_json!(StatsResponse {
        requests: STATS.requests.load(Ordering::Relaxed),
        errors: STATS.errors.load(Ordering::Relaxed),
        active_ws_sessions: state.notifier.send(ActiveSessionCount).await?,
        active_games: Game::active_game_count(&db).await?,
        active_db_connections: db.size() as usize,
        idle_db_connections: db.num_idle(),
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
