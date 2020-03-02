use actix_web::Result;
use chrono::Duration;
use chrono::{DateTime, Utc};
use diesel::prelude::*;

use crate::db;
use crate::errors::ServiceError;
use crate::invitations::Invitation;
use crate::schema::{games, invitations, users};

#[derive(Debug, Serialize, Deserialize, Queryable, Identifiable, AsChangeset)]
pub struct Game {
    pub id: i64,
    pub name: String,
    pub owner_id: i64,
    pub start_time: DateTime<Utc>,
    pub close_time: DateTime<Utc>,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize, Insertable)]
#[table_name = "games"]
pub struct CreateGame {
    pub name: String,
    #[serde(skip)]
    pub owner_id: i64,
    pub start_time: DateTime<Utc>,
    pub close_time: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct GameQuery {
    pub name: Option<String>,
    pub is_active: Option<bool>,
    pub owner_id: Option<i64>,
}

/// GameUser is used to query for invited users
#[derive(Serialize, Queryable)]
pub struct GameUser {
    pub user_id: i64,
    pub username: String,
    pub invitation_state: String,
}

#[derive(Debug, Deserialize)]
pub struct UserInvite {
    pub user_id: i64,
}

impl Game {
    pub fn create(new_game: CreateGame, conn: &db::Conn) -> Result<Game, ServiceError> {
        new_game.validate_duration()?;

        let game = conn.transaction::<Game, diesel::result::Error, _>(|| {
            let game: Game = diesel::insert_into(games::table)
                .values(&new_game)
                .get_result(conn)?;

            Invitation::new(game.id, game.owner_id)
                .accept()
                .save(conn)?;

            Ok(game)
        })?;

        Ok(game)
    }

    pub fn invite_user(&self, user_id: i64, conn: &db::Conn) -> Result<(), ServiceError> {
        let invite = Invitation::new(self.id, user_id);
        invite.save(conn)?;
        Ok(())
    }

    pub fn find_by_id(game_id: i64, conn: &db::Conn) -> Result<Game, ServiceError> {
        let game = games::table.filter(games::id.eq(game_id)).first(conn)?;
        Ok(game)
    }

    pub fn find_all(filter: GameQuery, conn: &db::Conn) -> Result<Vec<Game>, ServiceError> {
        let mut query = games::table.into_boxed();

        if filter.is_active.unwrap_or(false) {
            query = query.filter(games::close_time.gt(diesel::dsl::now));
        }

        if let Some(id) = filter.owner_id {
            query = query.filter(games::owner_id.eq(id));
        }

        if let Some(name) = filter.name {
            query = query.filter(games::name.like(format!("%{}%", name)));
        }

        let games = query.load::<Game>(conn)?;
        Ok(games)
    }

    pub fn find_users(game_id: i64, conn: &db::Conn) -> Result<Vec<GameUser>, ServiceError> {
        let res = invitations::table
            .inner_join(users::table)
            .filter(invitations::game_id.eq(game_id))
            .select((users::id, users::username, invitations::state))
            .load::<GameUser>(conn)?;
        Ok(res)
    }

    pub fn update(&self, conn: &db::Conn) -> Result<Game, ServiceError> {
        let game = diesel::update(self).set(self).get_result(conn)?;

        Ok(game)
    }

    pub fn delete(&self, conn: &db::Conn) -> Result<(), ServiceError> {
        diesel::delete(self).execute(conn)?;

        Ok(())
    }

    pub fn delete_by_id(game_id: i64, conn: &db::Conn) -> Result<(), ServiceError> {
        diesel::delete(games::table.filter(games::id.eq(game_id))).execute(conn)?;

        Ok(())
    }
}

impl CreateGame {
    fn validate_duration(&self) -> Result<(), ServiceError> {
        let duration: Duration = self.close_time.signed_duration_since(self.start_time);

        if duration.num_minutes() <= 0 {
            bad_request!("this game has not gone on long enough");
        }
        Ok(())
    }
}
