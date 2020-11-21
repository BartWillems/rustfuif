use std::collections::{HashMap, HashSet};

use actix::prelude::*;
use rand::{self, rngs::ThreadRng, Rng};

use crate::prices::PriceUpdate;
use crate::transactions::Transaction;
use crate::users::User;
use crate::websocket::queries::ActiveGamesResponse;

#[allow(dead_code)]
#[derive(Message)]
#[rtype(usize)]
pub struct Connect {
    pub addr: Recipient<Notification>,
    pub user: User,
    pub game_id: i64,
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct Disconnect {
    pub id: usize,
}

type GameId = i64;
type SessionId = usize;

#[derive(Debug)]
struct ConnectedUser {
    recipient: Recipient<Notification>,
    user: User,
}

#[allow(dead_code)]
impl ConnectedUser {
    fn new(recipient: Recipient<Notification>, user: User) -> Self {
        ConnectedUser { recipient, user }
    }
    fn send(&self, message: Notification) -> Result<(), actix::prelude::SendError<Notification>> {
        self.recipient.do_send(message)
    }

    fn user(&self) -> User {
        self.user.clone()
    }

    fn is_admin(&self) -> bool {
        self.user.is_admin
    }
}

/// `NotificationServer` manages price updates/new sales
pub struct NotificationServer {
    /// A hashmap containing the session ID as key and
    /// a `ConnectedUser` as value.
    /// The `ConnectedUser` contains the user and the actix
    /// recipient address.
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
            let _ = recipient.send(notification.clone());
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
            .for_each(|(_, user)| {
                let _ = user.send(notification.clone());
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
            .clone()
            .into_iter()
            .filter(|(_, sessions)| !sessions.is_empty())
            .map(|(game_id, sessions)| ActiveGamesResponse::new(game_id, sessions.len()))
            .collect()
    }

    /// return all connected users
    pub fn connected_users(&self) -> Vec<User> {
        self.sessions
            .values()
            .map(|session| session.user.clone())
            .collect()
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
    type Result = usize;

    fn handle(&mut self, msg: Connect, ctx: &mut Context<Self>) -> Self::Result {
        // register session with random id
        let session_id = self.rng.gen::<usize>();
        self.sessions
            .insert(session_id, ConnectedUser::new(msg.addr, msg.user));

        self.games
            .entry(msg.game_id)
            .or_insert_with(HashSet::new)
            .insert(session_id);

        ctx.notify(Notification::UserConnected(msg.game_id));

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
    UserConnected(GameId),
    /// Notify users in a certain game that someone left
    /// This is done by sending the ConnectionCount
    UserDisconnected(GameId),
    /// When a user leaves or joins a game, send the connection count
    /// This should be removed and the whole list of connected users should
    /// be sent instead.
    /// This is because I might implement a chat window later on
    ConnectionCount(usize),
}

#[derive(Message, Debug, Serialize, Clone)]
#[rtype(result = "()")]
pub struct Sale {
    pub game_id: i64,
    pub transactions: Vec<Transaction>,
}

impl Handler<Notification> for NotificationServer {
    type Result = ();

    fn handle(&mut self, notification: Notification, _: &mut Context<Self>) {
        match notification {
            Notification::NewSale(sale) => {
                self.notify_game(Notification::NewSale(sale.clone()), sale.game_id)
            }
            Notification::PriceUpdate(kind) => self.broadcast(Notification::PriceUpdate(kind)),
            Notification::UserConnected(game_id) => self.notify_game(
                Notification::ConnectionCount(self.users_in_game_count(game_id)),
                game_id,
            ),
            Notification::UserDisconnected(game_id) => self.notify_game(
                Notification::ConnectionCount(self.users_in_game_count(game_id)),
                game_id,
            ),
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
        if self.sessions.remove(&msg.id).is_some() {
            // remove session from all games
            for (game_id, sessions) in self.games.iter_mut() {
                if sessions.remove(&msg.id) {
                    ctx.notify(Notification::UserDisconnected(*game_id));
                    // this was the last user in the game
                    if sessions.is_empty() {
                        stale_game = Some(*game_id);
                    }
                }
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
    #[rtype(result = "Result<Vec<usize>, std::io::Error>")]
    pub struct InnerSessions;

    #[derive(Message)]
    #[rtype(usize)]
    pub struct InnerGamesCount;

    impl Handler<InnerSessions> for NotificationServer {
        type Result = Result<Vec<usize>, std::io::Error>;

        fn handle(&mut self, _: InnerSessions, _: &mut Context<Self>) -> Self::Result {
            let session_ids: Vec<usize> = self.sessions.keys().cloned().collect();
            Ok(session_ids)
        }
    }

    impl Handler<InnerGamesCount> for NotificationServer {
        type Result = usize;

        fn handle(&mut self, _: InnerGamesCount, _: &mut Context<Self>) -> Self::Result {
            self.games.len()
        }
    }

    async fn add_user(server: &Addr<NotificationServer>, game_id: i64) {
        let user = User {
            id: 1,
            username: String::from("admin"),
            is_admin: true,
            password: String::from("..."),
            created_at: None,
            updated_at: None,
        };
        server
            .send(Connect {
                addr: server.clone().recipient(),
                user: user.clone(),
                game_id,
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

        add_user(&server, 1).await;

        let users: Vec<usize> = server.send(InnerSessions).await.unwrap().unwrap();
        assert_eq!(1, users.len());

        let games_count: usize = server.send(InnerGamesCount).await.unwrap();
        assert_eq!(1, games_count);

        // connect the user to the same game
        add_user(&server, 1).await;

        let users: Vec<usize> = server.send(InnerSessions).await.unwrap().unwrap();
        assert_eq!(2, users.len());

        // The game count should stay the same
        let games_count: usize = server.send(InnerGamesCount).await.unwrap();
        assert_eq!(1, games_count);

        // connect the user to another game
        add_user(&server, 2).await;

        let users: Vec<usize> = server.send(InnerSessions).await.unwrap().unwrap();
        assert_eq!(3, users.len());

        let games_count: usize = server.send(InnerGamesCount).await.unwrap();
        assert_eq!(2, games_count);

        // now I will disconnect all the users
        // Both the games and the users should be completely cleaned up
        // resulting in a zero game count & zero user count
        for id in users {
            server.send(Disconnect { id }).await.unwrap();
        }

        let users: Vec<usize> = server.send(InnerSessions).await.unwrap().unwrap();
        assert_eq!(0, users.len());

        let games_count: usize = server.send(InnerGamesCount).await.unwrap();
        assert_eq!(0, games_count);
    }
}
