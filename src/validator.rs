use crate::errors::ServiceError;
use serde::de::DeserializeOwned;

#[derive(Deserialize)]
pub struct Validator<T>(T);

pub trait Validate<T> {
    fn validate(&self) -> Result<(), ServiceError>;
}

impl<T> Validator<T> {
    #[allow(dead_code)]
    pub fn new(i: T) -> Validator<T> {
        Validator::<T>(i)
    }
}

impl<T> Validator<T>
where
    T: Validate<T>,
    T: DeserializeOwned,
{
    pub fn validate(self) -> Result<T, ServiceError> {
        self.0.validate()?;
        Ok(self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    impl Validate<bool> for bool {
        fn validate(&self) -> Result<(), ServiceError> {
            if *self {
                return Ok(());
            }
            Err(ServiceError::BadRequest("invalid input".to_string()))
        }
    }

    #[test]
    fn invalid_value() {
        let invalid = Validator::new(false);

        assert!(invalid.validate().is_err());
    }

    #[test]
    fn valid_value() {
        let valid = Validator::new(true);

        assert!(valid.validate().is_ok());
    }
}
