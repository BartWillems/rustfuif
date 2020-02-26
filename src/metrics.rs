use std::sync::atomic::{AtomicUsize, Ordering};
use std::task::{Context, Poll};

use actix_service::{Service, Transform};
use actix_web::dev::{ServiceRequest, ServiceResponse};
use actix_web::{get, middleware, web, HttpRequest, HttpResponse};
use actix_web::{http, Error};
use futures::future::{ok, Either, Ready};
use std::rc::Rc;

use crate::server::Response;

pub type RequestCounter = AtomicUsize;

#[get("/metrics")]
pub async fn metrics(req: HttpRequest) -> Response {
    dbg!("dink request: {:?}", &req);

    // let ctr: Option<AtomicUsize> = HttpRequest::app_data(&req);

    let dink: Option<&AtomicUsize> = HttpRequest::app_data(&req);

    Ok(HttpResponse::Ok().json("ok"))
}

pub struct PerfCounter;

impl<S, B> Transform<S> for PerfCounter
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
        // self.0.fetch_add(1, Ordering::SeqCst);

        let counter = Rc::new(RequestCounter::new(0usize));

        ok(CheckPerfCounterMiddleware { service, counter })
    }
}

pub struct CheckPerfCounterMiddleware<S> {
    service: S,
    counter: Rc<RequestCounter>,
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
        self.counter.fetch_add(1, Ordering::SeqCst);

        // https://docs.rs/actix-web/2.0.0/actix_web/struct.App.html#method.app_data

        // actix_web::App::

        // actix_web::web::Data::

        // let dink: Option<&AtomicUsize> = HttpRequest::app_data(&req);

        // ServiceRequest::from_request(req: HttpRequest)

        // self.

        // HttpRequest::

        // ServiceRequest::

        debug!("middleware counter: {:?}", self.counter);

        debug!("{:?}", req);
        // debug!("{}", self);

        // self.call(req: ServiceRequest)

        Either::Left(self.service.call(req))

        // let is_logged_in = false; // Change this to see the change in outcome in the browser

        // if is_logged_in {
        //     Either::Left(self.service.call(req))
        // } else {
        //     // Don't forward to /login if we are already on /login
        //     if req.path() == "/login" {
        //         Either::Left(self.service.call(req))
        //     } else {
        //         Either::Right(ok(req.into_response(
        //             HttpResponse::Found()
        //                 .header(http::header::LOCATION, "/login")
        //                 .finish()
        //                 .into_body(),
        //         )))
        //     }
        // }
    }
}
