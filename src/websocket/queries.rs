use actix::prelude::*;

use crate::users::User;
use crate::websocket::server::NotificationServer;

#[derive(Message)]
#[rtype(usize)]
pub struct ActiveSessionCount;

impl Handler<ActiveSessionCount> for NotificationServer {
    type Result = usize;

    fn handle(&mut self, _: ActiveSessionCount, _: &mut Context<Self>) -> Self::Result {
        self.session_count()
    }
}

#[derive(Message)]
#[rtype(result = "Result<Vec<User>, std::io::Error>")]
pub struct ConnectedUsers;

impl Handler<ConnectedUsers> for NotificationServer {
    type Result = Result<Vec<User>, std::io::Error>;

    fn handle(&mut self, _: ConnectedUsers, _: &mut Context<Self>) -> Self::Result {
        Ok(self.connected_users())
    }
}

/// returns the active games and the amount of connected users
#[derive(Message)]
#[rtype(result = "Result<Vec<ActiveGamesResponse>, std::io::Error>")]
pub struct ActiveGames;

#[derive(Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ActiveGamesResponse {
    game_id: i64,
    session_count: usize,
}

impl ActiveGamesResponse {
    pub fn new(game_id: i64, session_count: usize) -> Self {
        ActiveGamesResponse {
            game_id,
            session_count,
        }
    }
}

impl Handler<ActiveGames> for NotificationServer {
    type Result = Result<Vec<ActiveGamesResponse>, std::io::Error>;

    fn handle(&mut self, _: ActiveGames, _: &mut Context<Self>) -> Self::Result {
        let games = self.games();
        Ok(games)
    }
}
