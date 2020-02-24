use actix_web::{error::ResponseError, HttpResponse};
use derive_more::Display;
use diesel::result::{DatabaseErrorKind, Error as DBError};
use std::convert::From;

#[derive(Debug, Display)]
pub enum ServiceError {
    #[display(fmt = "Internal Server Error")]
    InternalServerError,

    #[display(fmt = "BadRequest: {}", _0)]
    BadRequest(String),

    #[display(fmt = "Conflict: {}", _0)]
    Conflict(String),

    #[display(fmt = "Unauthorized")]
    Unauthorized,

    #[display(fmt = "Not Found")]
    NotFound,
}

// impl ResponseError trait allows to convert our errors into http responses with appropriate data
impl ResponseError for ServiceError {
    fn error_response(&self) -> HttpResponse {
        match self {
            ServiceError::InternalServerError => {
                HttpResponse::InternalServerError().json("Internal Server Error, Please try later")
            }
            ServiceError::BadRequest(ref message) => HttpResponse::BadRequest().json(message),
            ServiceError::Unauthorized => HttpResponse::Unauthorized().json("Unauthorized"),
            ServiceError::NotFound => HttpResponse::NotFound().json("Not Found"),
            ServiceError::Conflict(ref message) => HttpResponse::Conflict().json(message),
        }
    }
}

impl From<DBError> for ServiceError {
    fn from(error: DBError) -> ServiceError {
        error!("db error: {}", error);
        match error {
            DBError::NotFound => ServiceError::NotFound,
            DBError::DatabaseError(kind, info) => {
                if let DatabaseErrorKind::UniqueViolation = kind {
                    let message = info.details().unwrap_or_else(|| info.message()).to_string();
                    return ServiceError::Conflict(message);
                }
                ServiceError::InternalServerError
            }
            _ => ServiceError::InternalServerError,
        }
    }
}

impl From<r2d2::Error> for ServiceError {
    fn from(error: r2d2::Error) -> ServiceError {
        error!("r2d2 connection pool error: {}", error);
        ServiceError::InternalServerError
    }
}

impl<E: std::fmt::Debug> From<actix_threadpool::BlockingError<E>> for ServiceError {
    fn from(error: actix_threadpool::BlockingError<E>) -> ServiceError {
        error!("actix threadpool pool error: {}", error);
        ServiceError::InternalServerError
    }
}
