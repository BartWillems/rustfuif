use actix_web::error::Error as ActixError;
use actix_web::{error::ResponseError, HttpResponse};
use derive_more::Display;
use diesel::result::{DatabaseErrorKind, Error as DBError};
use redis::RedisError;
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

    #[display(fmt = "Forbidden: {}", _0)]
    Forbidden(String),

    #[display(fmt = "Payload Too Large")]
    PayloadTooLarge,
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
            ServiceError::Forbidden(ref message) => HttpResponse::Forbidden().json(message),
            ServiceError::Conflict(ref message) => HttpResponse::Conflict().json(message),
            ServiceError::PayloadTooLarge => {
                HttpResponse::PayloadTooLarge().json("Payload Too Large")
            }
        }
    }
}

impl From<ActixError> for ServiceError {
    fn from(error: ActixError) -> ServiceError {
        error!("actix http error: {}", error);
        ServiceError::InternalServerError
    }
}

impl From<actix::MailboxError> for ServiceError {
    fn from(error: actix::MailboxError) -> ServiceError {
        error!("actix mailbox error: {}", error);
        ServiceError::InternalServerError
    }
}

impl From<DBError> for ServiceError {
    fn from(error: DBError) -> ServiceError {
        error!("db error: {}", error);
        match error {
            DBError::NotFound => ServiceError::NotFound,
            DBError::DatabaseError(kind, info) => match kind {
                DatabaseErrorKind::UniqueViolation => {
                    debug!("unique violation");
                    ServiceError::Conflict(info.message().to_string())
                }
                DatabaseErrorKind::ForeignKeyViolation => {
                    debug!("foreign key violation");
                    ServiceError::BadRequest(info.message().to_string())
                }
                _ => ServiceError::InternalServerError,
            },
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

impl From<argon2::Error> for ServiceError {
    fn from(error: argon2::Error) -> ServiceError {
        error!("argon2 error: {}", error);
        match error {
            // these are the only error types that a user could influence
            argon2::Error::PwdTooShort => {
                ServiceError::BadRequest("password is too short".to_string())
            }
            argon2::Error::PwdTooLong => {
                ServiceError::BadRequest("password is too long".to_string())
            }
            _ => ServiceError::InternalServerError,
        };
        ServiceError::InternalServerError
    }
}

impl<E> From<actix_threadpool::BlockingError<E>> for ServiceError
where
    E: std::fmt::Debug,
    E: Into<ServiceError>,
{
    fn from(error: actix_threadpool::BlockingError<E>) -> ServiceError {
        match error {
            actix_threadpool::BlockingError::Error(e) => e.into(),
            actix_threadpool::BlockingError::Canceled => {
                error!("actix thread canceled");
                ServiceError::InternalServerError
            }
        }
    }
}

impl From<RedisError> for ServiceError {
    fn from(error: RedisError) -> ServiceError {
        error!("Redis error: {}", error);
        ServiceError::InternalServerError
    }
}

impl From<sqlx::Error> for ServiceError {
    fn from(error: sqlx::Error) -> ServiceError {
        debug!("SQLX error: {:?}", error);
        match error {
            sqlx::Error::Database(err) => {
                err.downcast_ref::<sqlx::postgres::PgDatabaseError>().into()
            }
            sqlx::Error::RowNotFound => ServiceError::NotFound,
            _ => ServiceError::InternalServerError,
        }
    }
}

impl From<&sqlx::postgres::PgDatabaseError> for ServiceError {
    fn from(error: &sqlx::postgres::PgDatabaseError) -> ServiceError {
        debug!("Postgres error: {:?}", error);
        match error.code() {
            "23505" => ServiceError::Conflict(error.to_string()),
            _ => ServiceError::InternalServerError,
        }
    }
}
