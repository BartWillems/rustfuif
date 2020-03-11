use actix_session::Session;

use crate::errors::ServiceError;

/// get the user_id of the current authenticated session
/// returns Unauthorized when no session is found
/// and ServerError when a session backend error occures
pub fn get_user_id(session: &Session) -> Result<i64, ServiceError> {
    let user_id: Option<i64> = session.get("user_id")?;

    match user_id {
        Some(id) => Ok(id),
        None => Err(ServiceError::Unauthorized),
    }
}

/// checks if the current user is an admin
pub fn is_admin(session: &Session) -> Result<bool, ServiceError> {
    let admin: Option<bool> = session.get("is_admin")?;

    Ok(admin.unwrap_or(false))
}
