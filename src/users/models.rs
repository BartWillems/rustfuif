use crate::schema::users;
use argon2::Config;
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use rand::Rng;
use regex::Regex;

use crate::db;
use crate::errors::ServiceError;

#[derive(Deserialize, AsChangeset, Insertable)]
#[table_name = "users"]
pub struct UserMessage {
    pub username: String,
    pub password: String,
}

#[derive(Serialize, Deserialize, Queryable, Insertable)]
pub struct User {
    pub id: i64,
    pub username: String,
    #[serde(skip_serializing, skip_deserializing)]
    pub password: String,
    pub is_admin: bool,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Serialize, Queryable)]
pub struct UserResponse {
    pub id: i64,
    pub username: String,
}

impl User {
    pub fn find_all(conn: &db::Conn) -> Result<Vec<Self>, ServiceError> {
        let users = users::table.load::<User>(conn)?;

        Ok(users)
    }

    pub fn find(id: i64, conn: &db::Conn) -> Result<Self, ServiceError> {
        let user = users::table.filter(users::id.eq(id)).first(conn)?;

        Ok(user)
    }

    pub fn find_by_username(username: String, conn: &db::Conn) -> Result<Self, ServiceError> {
        let user = users::table
            .filter(users::username.eq(username))
            .first(conn)?;

        Ok(user)
    }

    pub fn create(user: &mut UserMessage, conn: &db::Conn) -> Result<Self, ServiceError> {
        user.hash_password()?;

        let user: User = diesel::insert_into(users::table)
            .values(&*user)
            .get_result(conn)?;

        Ok(user)
    }

    pub fn update(id: i64, user: &mut UserMessage, conn: &db::Conn) -> Result<Self, ServiceError> {
        // this might be removed if I separate this with a "change_credentials" function dink
        user.hash_password()?;

        let user = diesel::update(users::table)
            .filter(users::id.eq(id))
            .set(&*user)
            .get_result(conn)?;

        Ok(user)
    }

    pub fn delete(id: i64, conn: &db::Conn) -> Result<(), ServiceError> {
        diesel::delete(users::table.filter(users::id.eq(id))).execute(conn)?;

        Ok(())
    }

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

impl UserMessage {
    // TODO: use generics or something to not duplicate this
    fn hash_password(&mut self) -> Result<(), ServiceError> {
        let salt: [u8; 32] = rand::thread_rng().gen();
        let config = Config::default();
        self.password = argon2::hash_encoded(self.password.as_bytes(), &salt, &config)?;
        Ok(())
    }
}

impl crate::validator::Validate<UserMessage> for UserMessage {
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

        if self.password.trim().len() < 8 {
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
        let user = UserMessage {
            username: String::from("a€$b"),
            password: String::from("hunter2boogaloo"),
        };

        assert!(Validator::new(user).validate().is_err());
    }

    #[test]
    fn empty_username() {
        let user = UserMessage {
            username: String::from(""),
            password: String::from("hunter2boogaloo"),
        };

        assert!(Validator::new(user).validate().is_err());
    }

    #[test]
    fn valid_username() {
        let user = UserMessage {
            username: String::from("rickybobby"),
            password: String::from("hunter2boogaloo"),
        };

        assert!(Validator::new(user).validate().is_ok());
    }

    #[test]
    fn valid_username_with_other_characters() {
        let user = UserMessage {
            username: String::from("a-b_c-0123"),
            password: String::from("hunter2boogaloo"),
        };

        assert!(Validator::new(user).validate().is_ok());
    }
}
