/// Return the request with HTTP 403
#[macro_export]
macro_rules! forbidden {
    ($message:expr) => {
        return Err(crate::errors::ServiceError::Forbidden($message.to_string()));
    };
}

/// Return the request with HTTP 400
#[macro_export]
macro_rules! bad_request {
    ($message:expr) => {
        return Err(crate::errors::ServiceError::BadRequest(
            $message.to_string(),
        ));
    };
}

/// Answer the request with HTTP 201 and the object as response body
#[macro_export]
macro_rules! http_created_json {
    ($object:expr) => {
        return Ok(actix_web::web::HttpResponse::Created().json($object));
    };
}

/// Answer the request with HTTP 200 and the object as response body
#[macro_export]
macro_rules! http_ok_json {
    ($object:expr) => {
        return Ok(actix_web::web::HttpResponse::Ok().json($object));
    };
}
