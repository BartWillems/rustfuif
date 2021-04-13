use std::fmt;

use actix_web::Result;
use chrono::{DateTime, Utc};
use sqlx::{Pool, Postgres};

use crate::games::GameResponse;
use crate::users::UserResponse;

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
#[derive(Debug, Serialize)]
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

#[derive(Debug, Serialize)]
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

    /// returns list of game invitations for a user
    #[tracing::instrument(name = "Invitation::find")]
    pub async fn my_invitations(
        user_id: i64,
        db: &Pool<Postgres>,
    ) -> Result<Vec<InvitationResponse>, sqlx::Error> {
        let rows = sqlx::query!(
            r#"
            SELECT invitations.id, invitations.state, games.id AS "game_id", games.name, games.start_time, games.close_time, games.beverage_count, users.id AS "user_id", users.username
            FROM invitations
            INNER JOIN games ON invitations.game_id = games.id
            INNER JOIN users ON games.owner_id = users.id
            WHERE 
                invitations.user_id = $1 
                AND games.close_time > NOW() 
                AND games.owner_id != $1
            ORDER BY games.start_time
            "#, 
            user_id
        ).fetch_all(db).await?;

        let invitations: Vec<InvitationResponse> = rows
            .into_iter()
            .map(|record| InvitationResponse {
                id: record.id,
                state: record.state,
                game: GameResponse {
                    id: record.game_id,
                    beverage_count: record.beverage_count,
                    name: record.name,
                    start_time: record.start_time,
                    close_time: record.close_time,
                    owner: UserResponse {
                        id: record.user_id,
                        username: record.username,
                    },
                },
            })
            .collect();

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
