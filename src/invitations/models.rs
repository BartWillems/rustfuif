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

impl Default for State {
    fn default() -> Self {
        State::PENDING
    }
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
pub struct Invitation {
    pub id: i64,
    pub game_id: i64,
    pub user_id: i64,
    pub state: String,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize, Insertable)]
#[table_name = "invitations"]
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

    pub fn save(&self, conn: &db::Conn) -> Result<Invitation, DBError> {
        diesel::insert_into(invitations::table)
            .values(self)
            .get_result::<Invitation>(conn)
    }

    pub fn accept(&mut self) -> &mut NewInvitation {
        self.state = State::ACCEPTED.to_string();
        self
    }
}

#[derive(Serialize, Queryable)]
pub struct InvitationResponse {
    pub id: i64,
    pub game: GameResponse,
    pub state: String,
}

impl Invitation {
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

    pub fn find_by_id(id: i64, conn: &db::Conn) -> Result<Invitation, ServiceError> {
        let invitation = invitations::table.find(id).first(conn)?;
        Ok(invitation)
    }

    /// get your game invites
    pub fn find(user_id: i64, conn: &db::Conn) -> Result<Vec<InvitationResponse>, ServiceError> {
        let invitations = invitations::table
            .inner_join(games::table.inner_join(users::table))
            .select((
                invitations::id,
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
            // do not show your own created games as invites
            .filter(games::owner_id.ne(user_id))
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
