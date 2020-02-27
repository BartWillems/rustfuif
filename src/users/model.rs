use crate::schema::users;
use argon2::Config;
use chrono::{DateTime, Utc};
use rand::Rng;

use crate::errors::ServiceError;

#[derive(Serialize, Deserialize, AsChangeset)]
#[table_name = "users"]
pub struct UserMessage {
    pub username: String,
    pub password: String,
}

#[derive(Serialize, Deserialize, Queryable, Insertable)]
pub struct User {
    pub id: i64,
    pub username: String,
    #[serde(skip_serializing)]
    pub password: String,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
}

impl User {
    pub fn hash_password(&mut self) -> Result<(), ServiceError> {
        let salt: [u8; 32] = rand::thread_rng().gen();
        let config = Config::default();

        self.password = argon2::hash_encoded(self.password.as_bytes(), &salt, &config)?;

        Ok(())
    }

    pub fn verify_password(&self, password: &[u8]) -> Result<bool, ServiceError> {
        let is_match = argon2::verify_encoded(&self.password, password)?;

        Ok(is_match)
    }
}
