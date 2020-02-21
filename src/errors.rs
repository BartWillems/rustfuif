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
            ServiceError::BadRequest(ref message) => {
                return HttpResponse::BadRequest().json(message);
            }
            ServiceError::Unauthorized => HttpResponse::Unauthorized().json("Unauthorized"),
            ServiceError::NotFound => HttpResponse::NotFound().json("Not Found"),
        }
    }
}

impl From<DBError> for ServiceError {
    fn from(error: DBError) -> ServiceError {
        // Right now we just care about UniqueViolation from diesel
        // But this would be helpful to easily map errors as our app grows
        error!("db error: {}", error);
        match error {
            DBError::NotFound => {
                return ServiceError::NotFound;
            }
            DBError::DatabaseError(kind, info) => {
                if let DatabaseErrorKind::UniqueViolation = kind {
                    let message = info.details().unwrap_or_else(|| info.message()).to_string();
                    return ServiceError::BadRequest(message);
                }

                // if let DatabaseErrorKind::
                ServiceError::InternalServerError
            }
            _ => ServiceError::InternalServerError,
        }
    }
}

// the trait `std::convert::From<r2d2::Error>` is not implemented for `errors::ServiceError`
impl From<r2d2::Error> for ServiceError {
    fn from(error: r2d2::Error) -> ServiceError {
        error!("r2d2 connection pool error: {}", error);
        match error {
            _ => ServiceError::InternalServerError,
        }
    }
}
