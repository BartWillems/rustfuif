use crate::schema::users;
use argon2::Config;
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use rand::Rng;

use crate::db;
use crate::errors::ServiceError;

#[derive(Serialize, Deserialize, AsChangeset, Insertable)]
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
    pub is_admin: bool,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
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

    pub fn find_by_email(username: String, conn: &db::Conn) -> Result<Self, ServiceError> {
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
