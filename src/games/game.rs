use actix_web::Result;
use chrono::Duration;
use chrono::{DateTime, Utc};
use diesel::prelude::*;

use crate::db;
use crate::errors::ServiceError;
use crate::invitations::{Invitation, State};
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

///
/// **POST /api/games**
///
/// This struct is used to create games.
///
/// The owner_id is is ignored when sent, as it's fetched from the user's session.
///
/// When the user isn't authenticated, Unauthorized(401) is returned,
/// otherwise Created(201) with the game object is returned.
///
/// ``` shell
/// curl --location --request POST 'localhost:8888/api/games' \
///     --header 'Content-Type: application/json' \
///     --data-raw '{
///	        "name": "feestje",
///	        "start_time": "2020-02-01T22:00:00Z",
///	        "close_time": "2020-02-01T23:59:59Z"
///     }'
/// ```
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

/// InviteMessage is what the client sends us to invite an
/// existing user to an existing game
///
/// **POST /api/games/{id}/users/invitations**
///
/// Example:
///
/// ``` shell
/// curl --location --request POST 'http://localhost:8080/api/games/1/users/invitations' \
/// --header 'Content-Type: application/json' \
/// --data-raw '{ "user_id": 2 }'
/// ```
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

    /// returns a list of users who have been invited for a game
    /// filter by changing the invitation state
    pub fn find_users(
        game_id: i64,
        state: Option<State>,
        conn: &db::Conn,
    ) -> Result<Vec<GameUser>, ServiceError> {
        let mut query = invitations::table
            .inner_join(users::table)
            .filter(invitations::game_id.eq(game_id))
            .into_boxed();

        if let Some(state) = state {
            query = query.filter(invitations::state.eq(state.to_string()));
        }

        let users = query
            .select((users::id, users::username, invitations::state))
            .load::<GameUser>(conn)?;

        Ok(users)
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

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use std::ops::Add;

    #[test]
    fn invalid_game_duration() {
        let time: DateTime<Utc> =
            DateTime::from_utc(NaiveDate::from_ymd(2020, 1, 1).and_hms(12, 0, 0), Utc);

        let smaller_time = time.add(Duration::hours(-1));

        let game_with_same_times = CreateGame {
            name: String::from("some_name"),
            owner_id: 1,
            start_time: time,
            close_time: time,
        };

        let game_with_smaller_end_time = CreateGame {
            name: String::from("some_name"),
            owner_id: 1,
            start_time: time,
            close_time: smaller_time,
        };

        let game_with_equal_bigger_end_time = CreateGame {
            name: String::from("some_name"),
            owner_id: 1,
            start_time: smaller_time,
            close_time: time,
        };

        assert!(game_with_same_times.validate_duration().is_err());
        assert!(game_with_smaller_end_time.validate_duration().is_err());

        assert!(game_with_equal_bigger_end_time.validate_duration().is_ok());
    }
}
