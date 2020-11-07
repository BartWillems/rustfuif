use actix_identity::Identity;

use crate::errors::ServiceError;
use crate::users::User;

pub fn get_user(id: &Identity) -> Result<User, ServiceError> {
    let user_str = id.identity().ok_or(ServiceError::Unauthorized)?;

    serde_json::from_str(&user_str).map_err(|e| {
        error!("unable to deserialize user: {}", e);
        ServiceError::Unauthorized
    })
}

pub fn verify_admin(id: &Identity) -> Result<(), ServiceError> {
    let user = get_user(id)?;

    if user.is_admin {
        return Ok(());
    }
    Err(ServiceError::Unauthorized)
}
