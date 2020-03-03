#[macro_export]
macro_rules! forbidden {
    ($message:expr) => {
        return Err(ServiceError::Forbidden($message.to_string()));
    };
}

#[macro_export]
macro_rules! bad_request {
    ($message:expr) => {
        return Err(ServiceError::BadRequest($message.to_string()));
    };
}

#[macro_export]
macro_rules! http_created_json {
    ($object:expr) => {
        return Ok(HttpResponse::Created().json($object));
    };
}

#[macro_export]
macro_rules! http_ok_json {
    ($object:expr) => {
        return Ok(HttpResponse::Ok().json($object));
    };
}
