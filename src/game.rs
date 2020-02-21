use actix_web::web::{Data, HttpResponse, Json};
use actix_web::{get, post, Result};
use diesel::insert_into;
use diesel::prelude::*;

// use std::time::Duration;

use crate::db;
use crate::errors::ServiceError;
// use crate::models;
use crate::schema::games;

#[derive(Debug, Serialize, Deserialize, Queryable)]
pub struct Game {
    pub id: i64,
    pub name: String,
    pub start_time: chrono::NaiveDateTime,
    pub duration: Option<i32>,
    // pub teams: Vec<models::Team>,
    // pub beverage_slots: Vec<models::Slot>,
    pub created_at: Option<chrono::NaiveDateTime>,
    pub updated_at: Option<chrono::NaiveDateTime>,
}

#[derive(Debug, Deserialize, Insertable)]
#[table_name = "games"]
pub struct CreateGame {
    pub name: String,
    pub start_time: chrono::NaiveDateTime,
    pub duration_in_seconds: i32,
    // pub teams: Vec<models::Team>,
    // pub beverage_amount: i8,
}

#[get("/")]
pub async fn get_games(pool: Data<db::Pool>) -> Result<HttpResponse, ServiceError> {
    use crate::schema::games::dsl::games;
    let conn = pool.get().unwrap();

    let dink = games.load::<Game>(&conn)?;

    Ok(HttpResponse::Ok().json(dink))
}

#[post("/")]
pub async fn create_game(
    game: Json<CreateGame>,
    pool: Data<db::Pool>,
) -> Result<HttpResponse, ServiceError> {
    use crate::schema::games::dsl::games;
    let conn = pool.get().unwrap();

    let game: Game = insert_into(games)
        .values(game.into_inner())
        .get_result(&conn)?;

    Ok(HttpResponse::Ok().json(game))
}

#[post("/{id}")]
pub async fn update_game(_game: Json<Game>, _pool: Data<db::Pool>) -> Result<String, ServiceError> {
    unimplemented!();
}
