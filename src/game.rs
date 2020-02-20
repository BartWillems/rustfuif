use actix_web::{post, web, Result};
use diesel::PgConnection;

use std::time::Duration;

use crate::db;
use crate::models;

#[derive(Debug, Serialize, Deserialize, Queryable)]
pub struct Game {
    pub id: i64,
    pub name: String,
    pub start_time: chrono::NaiveDateTime,
    pub duration: Duration,
    pub teams: Vec<models::Team>,
    pub beverage_slots: Vec<models::Slot>,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}

#[derive(Debug, Deserialize)]
pub struct CreateGame {
    pub name: String,
    pub start_time: chrono::NaiveDateTime,
    pub duration_in_seconds: i32,
    pub teams: Vec<models::Team>,
    pub beverage_amount: i8,
}

#[post("/")]
pub async fn create_game(game: web::Json<CreateGame>, pool: web::Data<db::Pool>) -> Result<String> {
    // use crate::schem
    // use crate::schema::
    // use crate::schema::game::dsl::game;

    use crate::schema::games::dsl::games;

    let conn: &PgConnection = pool.get_ref().get()?;

    Ok(format!("Creating game {}", game.name))
}
