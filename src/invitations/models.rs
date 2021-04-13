use std::fmt;

use actix_web::Result;
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use sqlx::{Pool, Postgres};

use crate::db;
use crate::errors::ServiceError;
use crate::games::GameResponse;
use crate::schema::{games, invitations, users};

/// The state shows wether a user has accepted, declined or not yet
/// responded to an invitation.
#[derive(Debug, Deserialize)]
pub enum State {
    PENDING,
    ACCEPTED,
    DECLINED,
}

impl Default for State {
    fn default() -> Self {
        State::PENDING
    }
}

impl fmt::Display for State {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

/// Game invite for a user.
/// When you create a game, you're also instantly invited and accepted
#[derive(Debug, Serialize, Queryable, Identifiable)]
#[serde(rename_all = "camelCase")]
pub struct Invitation {
    pub id: i64,
    pub game_id: i64,
    pub user_id: i64,
    pub state: String,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewInvitation {
    game_id: i64,
    user_id: i64,
    state: String,
}

impl NewInvitation {
    pub fn new(game_id: i64, user_id: i64) -> Self {
        NewInvitation {
            game_id,
            user_id,
            state: State::PENDING.to_string(),
        }
    }

    pub async fn save(&self, db: &Pool<Postgres>) -> Result<Invitation, sqlx::Error> {
        sqlx::query_as!(
            Invitation,
            "INSERT INTO invitations (game_id, user_id, state) VALUES ($1, $2, $3) RETURNING *;",
            self.game_id,
            self.user_id,
            self.state
        )
        .fetch_one(db)
        .await
    }

    pub fn accept(&mut self) -> &mut NewInvitation {
        self.state = State::ACCEPTED.to_string();
        self
    }
}

#[derive(Debug, Serialize, Queryable)]
pub struct InvitationResponse {
    pub id: i64,
    pub game: GameResponse,
    pub state: String,
}

impl Invitation {
    /// Update the invitation, returns the persisted invitation
    pub async fn update(&self, db: &Pool<Postgres>) -> Result<Invitation, sqlx::Error> {
        sqlx::query_as!(
            Invitation,
            "UPDATE invitations SET state = $1 WHERE id = $2 RETURNING *",
            self.state,
            self.id
        )
        .fetch_one(db)
        .await
    }

    pub async fn find_by_id(id: i64, db: &Pool<Postgres>) -> Result<Invitation, sqlx::Error> {
        sqlx::query_as!(Invitation, "SELECT * FROM invitations WHERE id = $1", id)
            .fetch_one(db)
            .await
    }

    /// get your game invites
    #[tracing::instrument(skip(conn))]
    pub fn find(user_id: i64, conn: &db::Conn) -> Result<Vec<InvitationResponse>, ServiceError> {
        let query = invitations::table
            .inner_join(games::table.inner_join(users::table))
            .select((
                invitations::id,
                (
                    games::id,
                    games::name,
                    games::start_time,
                    games::close_time,
                    games::beverage_count,
                    (users::id, users::username),
                ),
                invitations::state,
            ))
            .filter(invitations::user_id.eq(user_id))
            .filter(games::close_time.gt(diesel::dsl::now))
            // do not show your own created games as invites
            .filter(games::owner_id.ne(user_id))
            .into_boxed();

        let invitations = query
            .order(games::start_time)
            .load::<InvitationResponse>(conn)?;

        Ok(invitations)
    }

    /// mark a game as accepted, this does not automatically persist
    pub fn accept(&mut self) -> &mut Self {
        self.state = State::ACCEPTED.to_string();
        self
    }

    /// mark a game as accepted, this does not automatically persist
    pub fn decline(&mut self) -> &mut Self {
        self.state = State::DECLINED.to_string();
        self
    }
}

/// InviteMessage is what the client sends us to invite an
/// existing user to an existing game
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserInvite {
    pub user_id: i64,
}
