use actix_identity::Identity;

use crate::errors::ServiceError;
use crate::users::User;

pub fn get_user(id: &Identity) -> Result<User, ServiceError> {
    let user_str = id.identity().ok_or(ServiceError::Unauthorized)?;

    serde_json::from_str(&user_str).or_else(|e| {
        error!("unable to deserialize user: {}", e);
        return Err(ServiceError::Unauthorized);
    })
}
