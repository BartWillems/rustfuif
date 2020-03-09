use actix_web::Result;
use chrono::Duration;
use chrono::{DateTime, Utc};
use diesel::prelude::*;

use crate::db;
use crate::errors::ServiceError;
use crate::invitations::{Invitation, InvitationQuery, State};
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

/// GameFilter a struct that the client
/// can use to query for games.
#[derive(Debug, Deserialize)]
pub struct GameFilter {
    /// filter these games by %name%
    pub name: Option<String>,
    /// default false, set to true to hide games from the past
    pub hide_completed: Option<bool>,
    /// list games created by a specific user
    pub owner_id: Option<i64>,
}

/// A GameUser is a user who is invited for a game
#[derive(Serialize, Queryable)]
pub struct GameUser {
    pub user_id: i64,
    pub username: String,
    pub invitation_state: String,
}

impl Game {
    /// Creates a new game, saves it in the database and automatically invites and
    /// accepts the creator in a transaction.
    ///
    /// When something fails, the transaction rolls-back, returns an error
    /// and nothing will have happened.
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
        let game = games::table
            .filter(games::id.eq(game_id))
            .first::<Game>(conn)?;
        Ok(game)
    }

    pub fn find_all(filter: GameFilter, conn: &db::Conn) -> Result<Vec<Game>, ServiceError> {
        let mut query = games::table.into_boxed();

        if filter.hide_completed.unwrap_or(false) {
            query = query.filter(games::close_time.gt(diesel::dsl::now));
        }

        if let Some(id) = filter.owner_id {
            query = query.filter(games::owner_id.eq(id));
        }

        if let Some(name) = filter.name {
            query = query.filter(games::name.ilike(format!("%{}%", name)));
        }

        let games = query.load::<Game>(conn)?;
        Ok(games)
    }

    /// returns a list of users who have been invited for a game
    /// filter by changing the invitation state
    pub fn find_users(
        game_id: i64,
        filter: InvitationQuery,
        conn: &db::Conn,
    ) -> Result<Vec<GameUser>, ServiceError> {
        let mut query = invitations::table
            .inner_join(users::table)
            .filter(invitations::game_id.eq(game_id))
            .into_boxed();

        if let Some(state) = filter.state {
            query = query.filter(invitations::state.eq(state.to_string()));
        }

        let users = query
            .select((users::id, users::username, invitations::state))
            .load::<GameUser>(conn)?;

        Ok(users)
    }

    /// validates if a user is actually partaking in a game (invited and accepted)
    pub fn verify_user(game_id: i64, user_id: i64, conn: &db::Conn) -> Result<bool, ServiceError> {
        let res = invitations::table
            .filter(invitations::game_id.eq(game_id))
            .filter(invitations::user_id.eq(user_id))
            .filter(invitations::state.eq(State::ACCEPTED.to_string()))
            .select(invitations::user_id)
            .first::<i64>(conn)
            .optional()?;

        Ok(res.is_some())
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

    /// returns if the game is going on at the moment
    pub fn is_happening(&self) -> bool {
        let now = chrono::offset::Utc::now();
        if self.start_time < now && self.close_time > now {
            return true;
        }
        false
    }
}

/// minimum duration is 30 minutes
const MIN_GAME_SECONDS: i64 = 60 * 30;
/// maximum duration is 24 hours
const MAX_GAME_SECONDS: i64 = 60 * 60 * 24;

impl CreateGame {
    fn validate_duration(&self) -> Result<(), ServiceError> {
        let duration: Duration = self.close_time.signed_duration_since(self.start_time);
        if duration.num_seconds() <= MIN_GAME_SECONDS {
            bad_request!("this game has not gone on long enough, minimum duration is 30 minutes");
        }

        if duration.num_seconds() > MAX_GAME_SECONDS {
            bad_request!("the max duration of a game is 24 hours");
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
