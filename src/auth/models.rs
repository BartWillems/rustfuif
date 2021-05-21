use crate::errors::ServiceError;

#[derive(Deserialize)]
pub struct PasswordChange {
    pub old: String,
    pub new: String,
}

impl crate::validator::Validate<PasswordChange> for PasswordChange {
    fn validate(&self) -> Result<(), ServiceError> {
        if self.old == self.new {
            bad_request!("the new password can't be the same as the old password");
        }

        if self.new.len() < 8 {
            bad_request!("your password should be at least 8 characters long");
        }

        Ok(())
    }
}
