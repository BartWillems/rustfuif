use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::task::{Context, Poll};

use actix_service::{Service, Transform};
use actix_web::dev::{ServiceRequest, ServiceResponse};
use actix_web::web::{Data, ServiceConfig};
use actix_web::Error;
use actix_web::{get, HttpResponse};
use futures::future::{ok, Either, Ready};

use crate::server::Response;

pub fn _register(_cfg: &mut ServiceConfig) {
    let _metrics = Arc::new(Metrics::new());

    // TODO: figure out what type we need in order to use cfg.wrap()
    // cfg.wrap(MetricsMiddleware::new(Arc::clone(&metrics)));
    // cfg.data(Arc::clone(&metrics));
    // cfg.service(metrics_route);
    todo!();
}

// TODO: why the hell do you get garbage collected
#[derive(Serialize)]
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

#[get("/metrics")]
pub async fn metrics_route(metrics: Data<Arc<Metrics>>) -> Response {
    Ok(HttpResponse::Ok().json(metrics.into_inner()))
}

pub struct MetricsMiddleware(Arc<Metrics>);

impl MetricsMiddleware {
    pub fn new(dink: Arc<Metrics>) -> MetricsMiddleware {
        MetricsMiddleware(dink)
    }
}

impl<S, B> Transform<S> for MetricsMiddleware
where
    S: Service<Request = ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
{
    type Request = ServiceRequest;
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = CheckPerfCounterMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ok(CheckPerfCounterMiddleware {
            service,
            counter: self.0.clone(),
        })
    }
}

pub struct CheckPerfCounterMiddleware<S> {
    service: S,
    counter: Arc<Metrics>,
}

impl<S, B> Service for CheckPerfCounterMiddleware<S>
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
        // TODO: add counters for good & bad requests
        self.counter.requests.fetch_add(1, Ordering::SeqCst);

        debug!("counter: {:?}", self.counter.requests);

        // TODO: figure out how to fix this
        Either::Left(self.service.call(req))
    }
}
