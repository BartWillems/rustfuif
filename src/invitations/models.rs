use actix_web::Result;
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel::result::Error as DBError;

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
#[derive(Debug, Serialize, Insertable, Queryable, Identifiable, AsChangeset)]
#[primary_key(game_id, user_id)]
pub struct Invitation {
    pub game_id: i64,
    pub user_id: i64,
    pub state: String,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Serialize, Queryable)]
pub struct InvitationResponse {
    pub game: GameResponse,
    pub state: String,
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
    pub fn save(&self, conn: &db::Conn) -> Result<Invitation, DBError> {
        // This has to return the actual database error, because it's used in transactions.
        diesel::insert_into(invitations::table)
            .values(self)
            .get_result::<Invitation>(conn)
    }

    /// Update the invitation, returns the persisted invitation
    pub fn update(&self, conn: &db::Conn) -> Result<Invitation, ServiceError> {
        let invitation = diesel::update(self)
            .set(self)
            .get_result::<Invitation>(conn)?;
        Ok(invitation)
    }

    /// get your game invites
    pub fn find(user_id: i64, conn: &db::Conn) -> Result<Vec<InvitationResponse>, ServiceError> {
        let invitations = invitations::table
            .inner_join(users::table)
            .inner_join(games::table)
            .select((
                (
                    games::id,
                    games::name,
                    games::start_time,
                    games::close_time,
                    (users::id, users::username),
                ),
                invitations::state,
            ))
            .filter(invitations::user_id.eq(user_id))
            .filter(games::close_time.gt(diesel::dsl::now))
            .order(games::start_time)
            .load::<InvitationResponse>(conn)?;

        Ok(invitations)
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

/// InviteMessage is what the client sends us to invite an
/// existing user to an existing game
#[derive(Debug, Deserialize)]
pub struct UserInvite {
    pub user_id: i64,
}
