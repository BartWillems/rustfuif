use actix_web::Result;
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel::result::Error;

use crate::db;
use crate::errors::ServiceError;
use crate::schema::invitations;

/// The state shows wether a user has accepted, declined or not yet
/// responded to an invitation.
#[derive(Debug, Deserialize)]
pub enum State {
    PENDING,
    ACCEPTED,
    DECLINED,
}

/// InvitationQuery is used to filter invited users
#[derive(Debug, Deserialize)]
pub struct InvitationQuery {
    pub state: Option<State>,
}

impl std::fmt::Display for State {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

/// Game invite for a user.
/// When you create a game, you're also instantly invited and accepted
///
/// **GET /api/invitations**
///
/// When called, it shows the games you're invited to
///
/// Example:
///
/// ``` shell
/// curl --location --request GET 'http://localhost:8080/api/invitations'
/// [
///     {
///         "game_id": 1,
///         "user_id": 3,
///         "state": "ACCEPTED",
///         "created_at": "2020-03-02T19:53:33.977263Z",
///         "updated_at": null
///     }
/// ]
/// ```
#[derive(Debug, Serialize, Insertable, Queryable, Identifiable, AsChangeset)]
#[primary_key(game_id, user_id)]
pub struct Invitation {
    pub game_id: i64,
    pub user_id: i64,
    pub state: String,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
}

impl Invitation {
    pub fn new(game_id: i64, user_id: i64) -> Invitation {
        Invitation {
            game_id,
            user_id,
            state: State::PENDING.to_string(),
            created_at: None,
            updated_at: None,
        }
    }

    /// Store an invitation in the database, returns the persisted invitation, or a database error
    pub fn save(&self, conn: &db::Conn) -> Result<Invitation, Error> {
        let invitation: Invitation = diesel::insert_into(invitations::table)
            .values(self)
            .get_result(conn)?;
        Ok(invitation)
    }

    /// Update the invitation, returns the persisted invitation
    pub fn update(&self, conn: &db::Conn) -> Result<Invitation, ServiceError> {
        let invitation = diesel::update(self).set(self).get_result(conn)?;

        Ok(invitation)
    }

    /// get your game invites
    pub fn find(user_id: i64, conn: &db::Conn) -> Result<Vec<Invitation>, ServiceError> {
        let invites = invitations::table
            .filter(invitations::user_id.eq(user_id))
            .load::<Invitation>(conn)?;

        Ok(invites)
    }

    /// mark a game as accepted, this does not automatically persist
    pub fn accept(&mut self) -> &mut Invitation {
        self.state = State::ACCEPTED.to_string();
        self
    }

    /// mark a game as accepted, this does not automatically persist
    pub fn decline(&mut self) -> &mut Invitation {
        self.state = State::DECLINED.to_string();
        self
    }
}
