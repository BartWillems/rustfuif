use actix_web::Result;
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel::result::Error;

use crate::db;
use crate::errors::ServiceError;
use crate::schema::invitations;

#[derive(Debug, Deserialize)]
pub enum State {
    PENDING,
    ACCEPTED,
    DECLINED,
}

impl std::fmt::Display for State {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Debug, Serialize, Insertable, Queryable, Identifiable, AsChangeset)]
#[primary_key(game_id, user_id)]
pub struct Invitation {
    pub game_id: i64,
    pub user_id: i64,
    pub state: String,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
pub struct InviteMessage {
    pub game_id: i64,
    pub user_id: i64,
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

    pub fn save(&self, conn: &db::Conn) -> Result<Invitation, Error> {
        let invitation: Invitation = diesel::insert_into(invitations::table)
            .values(self)
            .get_result(conn)?;
        Ok(invitation)
    }

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

    pub fn accept(&mut self) -> &mut Invitation {
        self.state = State::ACCEPTED.to_string();
        self
    }

    pub fn decline(&mut self) -> &mut Invitation {
        self.state = State::DECLINED.to_string();
        self
    }
}

impl From<InviteMessage> for Invitation {
    fn from(i: InviteMessage) -> Invitation {
        Invitation::new(i.game_id, i.user_id)
    }
}
