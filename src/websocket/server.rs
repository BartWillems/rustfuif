use std::collections::{HashMap, HashSet};

use actix::prelude::*;
use rand::{self, rngs::ThreadRng, Rng};

use crate::market::MarketStatus;
use crate::transactions::Transaction;
use crate::users::User;
use crate::websocket::queries::ActiveGamesResponse;

#[derive(Debug, Copy, Serialize, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GameId(pub i64);

#[derive(Debug, Copy, Serialize, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, MessageResponse)]
pub struct SessionId(pub usize);

impl Default for SessionId {
    /// Used to create an empty session
    fn default() -> Self {
        SessionId(0)
    }
}

#[derive(Debug, Serialize, Clone, Copy)]
pub enum ConnectionType {
    GameConnection(GameId),
    AdminConnection,
}

#[derive(Message)]
#[rtype(SessionId)]
pub struct Connect {
    pub addr: Recipient<Notification>,
    pub user: User,
    pub connection_type: ConnectionType,
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct Disconnect {
    pub id: SessionId,
}

#[derive(Debug)]
struct ConnectedUser {
    recipient: Recipient<Notification>,
    user: User,
}

impl ConnectedUser {
    fn new(recipient: Recipient<Notification>, user: User) -> Self {
        ConnectedUser { recipient, user }
    }

    fn send(&self, message: Notification) -> Result<(), actix::prelude::SendError<Notification>> {
        self.recipient.do_send(message)
    }

    fn user(&self) -> &User {
        &self.user
    }

    fn is_admin(&self) -> bool {
        self.user.is_admin
    }
}

/// `NotificationServer` manages price updates/new sales
pub struct NotificationServer {
    /// A hashmap containing the session ID as key and a `ConnectedUser` as value.
    /// The `ConnectedUser` contains the user and the actix  recipient address.
    sessions: HashMap<SessionId, ConnectedUser>,
    games: HashMap<GameId, HashSet<SessionId>>,
    rng: ThreadRng,
}

#[allow(dead_code)]
impl NotificationServer {
    pub fn new() -> Self {
        NotificationServer {
            sessions: HashMap::new(),
            games: HashMap::new(),
            rng: rand::thread_rng(),
        }
    }

    /// send a message to all connected users
    pub fn broadcast(&self, notification: Notification) {
        for (_, recipient) in self.sessions.iter() {
            if let Err(error) = recipient.send(notification.clone()) {
                error!(
                    "Unable to notify {} about {:?}, error: {}",
                    recipient.user(),
                    notification,
                    error
                );
            }
        }
    }

    /// send a message to all connected users in a game
    pub fn notify_game(&self, notification: Notification, game_id: GameId) {
        if let Some(sessions) = self.games.get(&game_id) {
            for id in sessions {
                if let Some(addr) = self.sessions.get(id) {
                    let _ = addr.send(notification.clone());
                }
            }
        }
    }

    /// send a message to all connected administrators
    pub fn notify_administrators(&self, notification: Notification) {
        self.sessions
            .iter()
            .filter(|&(_, user)| user.is_admin())
            .for_each(|(_, admin)| {
                let _ = admin.send(notification.clone());
            });
    }

    pub fn notify_user(&self, notification: Notification, user_id: i64) {
        self.sessions
            .iter()
            .filter(|&(_, connection)| connection.user.id == user_id)
            .for_each(|(_, connection)| {
                connection.send(notification.clone()).ok();
            });
    }

    /// returns the number of connected users
    pub fn session_count(&self) -> usize {
        self.sessions.len()
    }

    /// returns the number of connected users for a given game
    pub fn users_in_game_count(&self, game_id: GameId) -> usize {
        self.games
            .get(&game_id)
            .map(|sessions| sessions.len())
            .unwrap_or(0)
    }

    /// returns a hashmap with active games and their current connected player count
    pub fn games(&self) -> Vec<ActiveGamesResponse> {
        self.games
            .iter()
            .filter(|(_, sessions)| !sessions.is_empty())
            .map(|(game_id, sessions)| ActiveGamesResponse::new(game_id.0, sessions.len()))
            .collect()
    }

    /// return all connected users
    pub fn connected_users(&self) -> Vec<User> {
        self.sessions
            .values()
            .map(|session| session.user.clone())
            .collect()
    }

    pub fn connection_change(&self, connection_type: ConnectionType) {
        match connection_type {
            ConnectionType::GameConnection(game_id) => {
                self.notify_game(
                    Notification::ConnectionCount(self.users_in_game_count(game_id)),
                    game_id,
                );

                // Also notify the administrators
                self.notify_administrators(Notification::ConnectedUsers(self.connected_users()));
                self.notify_administrators(Notification::ActiveGames(self.games()));
            }
            ConnectionType::AdminConnection => {
                self.notify_administrators(Notification::ConnectedUsers(self.connected_users()));
            }
        };
    }
}

/// Make actor from `NotificationServer`
impl Actor for NotificationServer {
    /// We are going to use simple Context, we just need ability to communicate
    /// with other actors.
    type Context = Context<Self>;
}

/// Handler for Connect message.
///
/// Register new session and assign unique id to this session
impl Handler<Connect> for NotificationServer {
    type Result = SessionId;

    fn handle(&mut self, msg: Connect, ctx: &mut Context<Self>) -> Self::Result {
        // register session with random id
        let session_id = SessionId(self.rng.gen::<usize>());
        self.sessions
            .insert(session_id, ConnectedUser::new(msg.addr, msg.user));

        match msg.connection_type {
            ConnectionType::GameConnection(game_id) => {
                self.games
                    .entry(game_id)
                    .or_insert_with(HashSet::new)
                    .insert(session_id);
                ctx.notify(Notification::UserConnected(ConnectionType::GameConnection(
                    game_id,
                )));
            }
            ConnectionType::AdminConnection => {
                ctx.notify(Notification::UserConnected(ConnectionType::AdminConnection));
            }
        };

        debug!("new connection!");
        debug!("sessions count: {}", self.sessions.len());
        debug!("games count: {}", self.games.len());

        session_id
    }
}

#[derive(Message, Debug, Serialize, Clone)]
#[rtype(result = "()")]
pub enum Notification {
    /// Notify users in a game when a new sale happened
    NewSale(Sale),
    /// Notify all connected users that he prices are updated
    PriceUpdate(PriceUpdate),
    /// Notify users in a certain game that someone joined
    /// This is done by sending the ConnectionCount
    UserConnected(ConnectionType),
    /// Notify users in a certain game that someone left
    /// This is done by sending the ConnectionCount
    UserDisconnected(ConnectionType),
    /// When a user leaves or joins a game, send the connection count
    /// This should be removed and the whole list of connected users should
    /// be sent instead.
    /// This is because I might implement a chat window later on
    ConnectionCount(usize),
    ConnectedUsers(Vec<User>),
    ActiveGames(Vec<ActiveGamesResponse>),
}

#[derive(Message, Debug, Serialize, Clone)]
#[rtype(result = "()")]
pub struct Sale {
    pub game_id: GameId,
    pub transactions: Vec<Transaction>,
}

#[derive(Debug, Serialize, Clone, Copy)]
pub struct PriceUpdate {
    pub market_status: MarketStatus,
    pub game_id: GameId,
}

impl Handler<Notification> for NotificationServer {
    type Result = ();

    fn handle(&mut self, notification: Notification, _: &mut Context<Self>) {
        match notification {
            Notification::NewSale(sale) => {
                self.notify_game(Notification::NewSale(sale.clone()), sale.game_id)
            }
            Notification::PriceUpdate(update) => self.notify_game(notification, update.game_id),
            Notification::UserConnected(connection_type) => self.connection_change(connection_type),
            Notification::UserDisconnected(connection_type) => {
                self.connection_change(connection_type)
            }
            _ => (),
        }
    }
}

/// Handler for Disconnect message.
impl Handler<Disconnect> for NotificationServer {
    type Result = ();

    fn handle(&mut self, msg: Disconnect, ctx: &mut Context<Self>) {
        let mut stale_game: Option<GameId> = None;
        // remove address
        if let Some(session) = self.sessions.remove(&msg.id) {
            // remove session from all games
            for (game_id, game_sessions) in self.games.iter_mut() {
                if game_sessions.remove(&msg.id) {
                    ctx.notify(Notification::UserDisconnected(
                        ConnectionType::GameConnection(*game_id),
                    ));
                    // this was the last user in the game
                    if game_sessions.is_empty() {
                        stale_game = Some(*game_id);
                    }
                }
            }

            if session.is_admin() {
                ctx.notify(Notification::UserDisconnected(
                    ConnectionType::AdminConnection,
                ));
            }
        }

        // the game has no more players, remove it from the list
        if let Some(game_id) = stale_game {
            self.games.remove(&game_id);
        }

        debug!("user disconnected");
        debug!("sessions count: {}", self.sessions.len());
        debug!("games count: {}", self.games.len());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Message)]
    #[rtype(result = "Result<Vec<SessionId>, std::io::Error>")]
    pub struct InnerSessions;

    #[derive(Message)]
    #[rtype(usize)]
    pub struct InnerGamesCount;

    impl Handler<InnerSessions> for NotificationServer {
        type Result = Result<Vec<SessionId>, std::io::Error>;

        fn handle(&mut self, _: InnerSessions, _: &mut Context<Self>) -> Self::Result {
            let session_ids: Vec<SessionId> = self.sessions.keys().cloned().collect();
            Ok(session_ids)
        }
    }

    impl Handler<InnerGamesCount> for NotificationServer {
        type Result = usize;

        fn handle(&mut self, _: InnerGamesCount, _: &mut Context<Self>) -> Self::Result {
            self.games.len()
        }
    }

    async fn add_user(
        server: &Addr<NotificationServer>,
        connection_type: ConnectionType,
        is_admin: bool,
    ) {
        let user = User {
            id: 1,
            username: String::from("admin"),
            is_admin,
            password: String::from("..."),
            created_at: None,
            updated_at: None,
        };
        server
            .send(Connect {
                addr: server.clone().recipient(),
                user: user.clone(),
                connection_type,
            })
            .await
            .unwrap();
    }

    /// This test should prove that the websocket sessions get correctly cleaned up
    ///
    /// The games hashmap(HashMap<GameId, HashSet<SessionId>>) should remove the GameId key correctly when the containing
    /// session count reaches 0
    #[actix_rt::test]
    async fn session_cleanup() {
        let server = NotificationServer::new().start();

        add_user(&server, ConnectionType::GameConnection(GameId(1)), true).await;

        let users: Vec<SessionId> = server.send(InnerSessions).await.unwrap().unwrap();
        assert_eq!(1, users.len());

        let games_count: usize = server.send(InnerGamesCount).await.unwrap();
        assert_eq!(1, games_count);

        // connect the user to the same game
        add_user(&server, ConnectionType::GameConnection(GameId(1)), true).await;

        let users: Vec<SessionId> = server.send(InnerSessions).await.unwrap().unwrap();
        assert_eq!(2, users.len());

        // The game count should stay the same
        let games_count: usize = server.send(InnerGamesCount).await.unwrap();
        assert_eq!(1, games_count);

        // connect the user to another game
        add_user(&server, ConnectionType::GameConnection(GameId(2)), true).await;

        let users: Vec<SessionId> = server.send(InnerSessions).await.unwrap().unwrap();
        assert_eq!(3, users.len());

        let games_count: usize = server.send(InnerGamesCount).await.unwrap();
        assert_eq!(2, games_count);

        // now I will disconnect all the users
        // Both the games and the users should be completely cleaned up
        // resulting in a zero game count & zero user count
        for id in users {
            server.send(Disconnect { id }).await.unwrap();
        }

        let users: Vec<SessionId> = server.send(InnerSessions).await.unwrap().unwrap();
        assert_eq!(0, users.len());

        let games_count: usize = server.send(InnerGamesCount).await.unwrap();
        assert_eq!(0, games_count);

        // connect an admin user
        // the games count should not change
        add_user(&server, ConnectionType::AdminConnection, true).await;
        let users: Vec<SessionId> = server.send(InnerSessions).await.unwrap().unwrap();
        assert_eq!(1, users.len());

        let games_count: usize = server.send(InnerGamesCount).await.unwrap();
        assert_eq!(0, games_count);
    }
}
