use argon2::Config;
use chrono::{DateTime, Utc};
use rand::Rng;
use regex::Regex;
use sqlx::{Pool, Postgres};

use crate::errors::ServiceError;

#[derive(Deserialize)]
pub struct Credentials {
    pub username: String,
    pub password: String,
}

impl std::fmt::Debug for Credentials {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UserMessage")
            .field("username", &self.username)
            .field("password", &"redacted")
            .finish()
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct User {
    pub id: i64,
    pub username: String,
    #[serde(skip_serializing, skip_deserializing)]
    pub password: String,
    pub is_admin: bool,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, sqlx::Type)]
pub struct UserResponse {
    pub id: i64,
    pub username: String,
}

#[derive(Debug, Deserialize)]
pub struct Filter {
    /// filter users by %name%
    pub username: Option<String>,
    /// skips users why are invited for game by ID
    pub not_in_game: Option<i64>,
}

impl User {
    #[tracing::instrument(name = "user::find_all")]
    pub async fn find_all(filter: Filter, db: &Pool<Postgres>) -> Result<Vec<Self>, sqlx::Error> {
        let users = sqlx::query_as!(
            User,
            "SELECT * FROM users WHERE username ilike $1",
            format!("%{}%", filter.username.unwrap_or_default())
        )
        .fetch_all(db)
        .await?;

        Ok(users)
    }

    #[tracing::instrument(name = "user::find")]
    pub async fn find(id: i64, db: &Pool<Postgres>) -> Result<Self, sqlx::Error> {
        let user = sqlx::query_as!(User, "SELECT * FROM users WHERE id = $1", id)
            .fetch_one(db)
            .await?;

        Ok(user)
    }

    #[tracing::instrument(name = "user::find_by_name")]
    pub async fn find_by_name(username: &str, db: &Pool<Postgres>) -> Result<Self, sqlx::Error> {
        let user = sqlx::query_as!(User, "SELECT * FROM users WHERE username = $1", username)
            .fetch_one(db)
            .await?;

        Ok(user)
    }

    /// Store the user in the database after hashing it's password
    #[tracing::instrument(name = "user::create")]
    pub async fn create(user: &mut Credentials, db: &Pool<Postgres>) -> Result<Self, ServiceError> {
        user.hash_password()?;

        let user = sqlx::query_as!(
            User,
            r"INSERT INTO users (username, password) VALUES ($1, $2) RETURNING *;",
            user.username,
            user.password
        )
        .fetch_one(db)
        .await?;

        Ok(user)
    }

    /// Hash and store the user's changed password to the database
    #[tracing::instrument(name = "user::update_password", skip(password))]
    pub async fn update_password(
        &mut self,
        password: String,
        db: &Pool<Postgres>,
    ) -> Result<(), ServiceError> {
        self.set_password(password).hash_password()?;

        sqlx::query!(
            "UPDATE users SET password = $1 WHERE id = $2",
            &self.password,
            self.id
        )
        .execute(db)
        .await?;

        Ok(())
    }

    /// return the total amount of registered users
    #[tracing::instrument(name = "user::count")]
    pub async fn count(db: &Pool<Postgres>) -> Result<i64, sqlx::Error> {
        let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users")
            .fetch_one(db)
            .await?;

        Ok(row.0)
    }

    /// Returns Ok(()) if the user's hashed password matches the given password
    pub fn verify_password(&self, password: &[u8]) -> Result<(), ServiceError> {
        let is_match = argon2::verify_encoded(&self.password, password)?;

        if !is_match {
            return Err(ServiceError::Unauthorized);
        }

        Ok(())
    }
}

impl std::fmt::Display for User {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.username)
    }
}

trait PasswordHash {
    fn hash_password(&mut self) -> Result<(), ServiceError> {
        let salt: [u8; 32] = rand::thread_rng().gen();
        let config = Config::default();
        let hash = argon2::hash_encoded(self.password().as_bytes(), &salt, &config)?;
        self.set_password(hash);
        Ok(())
    }

    /// Get the current password
    fn password(&self) -> &str;
    fn set_password(&mut self, password: String) -> &mut Self;
}

impl PasswordHash for User {
    fn password(&self) -> &str {
        &self.password
    }

    fn set_password(&mut self, password: String) -> &mut Self {
        self.password = password;
        self
    }
}

impl PasswordHash for Credentials {
    fn password(&self) -> &str {
        &self.password
    }

    fn set_password(&mut self, password: String) -> &mut Self {
        self.password = password;
        self
    }
}

impl crate::validator::Validate<Credentials> for Credentials {
    fn validate(&self) -> Result<(), ServiceError> {
        if self.username.trim().is_empty() {
            bad_request!("username is too short");
        }

        if self.username.trim().len() > 20 {
            bad_request!("username is too long, max 20 characters");
        }

        let pattern: Regex = Regex::new(r"^[0-9A-Za-z-_]+$").unwrap();

        if !pattern.is_match(&self.username) {
            bad_request!("username can only contain letters, numbers, '-' and '_'");
        }

        if self.password.len() < 8 {
            bad_request!("your password should at least be 8 characters long");
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::validator::Validator;

    #[test]
    /// the user password should never be exposed through the api
    fn password_should_not_leak() {
        let password = "password";
        let user = User {
            id: 1,
            username: "".to_string(),
            password: password.to_string(),
            is_admin: false,
            created_at: None,
            updated_at: None,
        };

        let serialized = serde_json::to_string(&user).unwrap();

        assert_eq!(serialized.contains(password), false);
    }

    #[test]
    fn invalid_username() {
        let user = Credentials {
            username: String::from("a€$b"),
            password: String::from("hunter2boogaloo"),
        };

        assert!(Validator::new(user).validate().is_err());
    }

    #[test]
    fn empty_username() {
        let user = Credentials {
            username: String::from(""),
            password: String::from("hunter2boogaloo"),
        };

        assert!(Validator::new(user).validate().is_err());
    }

    #[test]
    fn valid_username() {
        let user = Credentials {
            username: String::from("rickybobby"),
            password: String::from("hunter2boogaloo"),
        };

        assert!(Validator::new(user).validate().is_ok());
    }

    #[test]
    fn valid_username_with_other_characters() {
        let user = Credentials {
            username: String::from("a-b_c-0123"),
            password: String::from("hunter2boogaloo"),
        };

        assert!(Validator::new(user).validate().is_ok());
    }

    #[test]
    fn incorrect_password() {
        let mut user = User {
            id: 1,
            is_admin: true,
            username: String::from("admin"),
            password: String::from("admin"),
            created_at: None,
            updated_at: None,
        };

        user.hash_password().unwrap();

        assert!(user.verify_password(b"admin").is_ok());
        assert!(user.verify_password(b"not-admin").is_err());
        assert_ne!(user.password(), "admin");
    }
}
