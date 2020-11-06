use std::collections::{HashMap, HashSet};
use std::sync::{mpsc, Arc};
use std::thread;

use actix::prelude::*;
use rand::{self, rngs::ThreadRng, Rng};

use crate::transactions::Transaction;
use crate::users::User;

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

#[derive(Message)]
#[rtype(usize)]
pub enum Query {
    ActiveSessions,
}

#[derive(Message)]
#[rtype(result = "Result<Vec<User>, std::io::Error>")]
pub struct ConnectedUsers;

type GameId = i64;
type SessionId = usize;

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

/// `TransactionServer` manages price updates/new sales
pub struct TransactionServer {
    sessions: HashMap<SessionId, ConnectedUser>,
    games: HashMap<GameId, HashSet<SessionId>>,
    rng: ThreadRng,
    updates: mpsc::Sender<Notification>,
}

#[allow(dead_code)]
impl TransactionServer {
    pub fn new(updates: mpsc::Sender<Notification>) -> Self {
        TransactionServer {
            sessions: HashMap::new(),
            games: HashMap::new(),
            rng: rand::thread_rng(),
            updates,
        }
    }
    /// Notify all players of a game that a sale happened
    pub fn notify_sale(&self, sale: Sale) {
        if let Some(sessions) = self.games.get(&sale.game_id) {
            for id in sessions {
                if let Some(addr) = self.sessions.get(id) {
                    let _ = addr.send(Notification::NewSale(sale.clone()));
                }
            }
        }
    }

    pub fn notify_price_update(&self) {
        for (_, recipient) in self.sessions.iter() {
            let _ = recipient.send(Notification::PriceUpdate);
        }
    }

    pub fn notify_connection_change(&self, game_id: GameId) {
        if let Some(sessions) = self.games.get(&game_id) {
            for id in sessions {
                if let Some(addr) = self.sessions.get(id) {
                    // let _ = addr.send(Notification::NewSale(sale.clone()));
                    let _ = addr.send(Notification::ConnectionCount(
                        self.users_in_game_count(game_id),
                    ));
                }
            }
        }
    }

    /// Listens for updates made by the beursfuif
    /// Eg: a purchase is made, or the prices have been updated
    pub fn listener(server: Arc<Addr<TransactionServer>>, rx: mpsc::Receiver<Notification>) {
        thread::spawn(move || {
            for notification in rx {
                server.do_send(notification.clone());
            }
        });
    }

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
    pub fn games(&self) -> HashMap<GameId, usize> {
        self.games
            .clone()
            .into_iter()
            .filter(|(_, sessions)| sessions.len() > 0)
            .map(|(game_id, sessions)| (game_id, sessions.len()))
            .collect()
    }

    pub fn connected_users(&self) -> Vec<User> {
        let mut users: Vec<User> = Vec::new();
        for session in self.sessions.values() {
            users.push(session.user.clone());
        }
        users
    }
}

/// Make actor from `TransactionServer`
impl Actor for TransactionServer {
    /// We are going to use simple Context, we just need ability to communicate
    /// with other actors.
    type Context = Context<Self>;
}

/// Handler for Connect message.
///
/// Register new session and assign unique id to this session
impl Handler<Connect> for TransactionServer {
    type Result = usize;

    fn handle(&mut self, msg: Connect, _: &mut Context<Self>) -> Self::Result {
        // register session with random id
        let id = self.rng.gen::<usize>();
        self.sessions
            .insert(id, ConnectedUser::new(msg.addr, msg.user));

        self.games
            .entry(msg.game_id)
            .or_insert_with(HashSet::new)
            .insert(id);

        let _ = self.updates.send(Notification::UserConnected(msg.game_id));

        id
    }
}

impl Handler<Query> for TransactionServer {
    type Result = usize;

    fn handle(&mut self, msg: Query, _: &mut Context<Self>) -> Self::Result {
        match msg {
            Query::ActiveSessions => self.session_count(),
        }
    }
}

impl Handler<ConnectedUsers> for TransactionServer {
    type Result = Result<Vec<User>, std::io::Error>;

    fn handle(&mut self, _: ConnectedUsers, _: &mut Context<Self>) -> Self::Result {
        let users = self.connected_users();
        Ok(users)
    }
}

#[derive(Message, Debug, Serialize, Clone)]
#[rtype(result = "()")]
pub enum Notification {
    NewSale(Sale),
    PriceUpdate,
    UserConnected(GameId),
    UserDisconnected(GameId),
    ConnectionCount(usize),
}

#[derive(Message, Debug, Serialize, Clone)]
#[rtype(result = "()")]
pub struct Sale {
    pub game_id: i64,
    pub transactions: Vec<Transaction>,
}

impl Handler<Notification> for TransactionServer {
    type Result = ();

    fn handle(&mut self, notification: Notification, _: &mut Context<Self>) {
        match notification {
            Notification::NewSale(sale) => {
                self.notify_sale(sale);
            }
            Notification::PriceUpdate => self.notify_price_update(),
            Notification::UserConnected(game_id) => self.notify_connection_change(game_id),
            Notification::UserDisconnected(game_id) => self.notify_connection_change(game_id),
            _ => (),
        }
    }
}

/// Handler for Disconnect message.
impl Handler<Disconnect> for TransactionServer {
    type Result = ();

    fn handle(&mut self, msg: Disconnect, _: &mut Context<Self>) {
        // remove address
        if self.sessions.remove(&msg.id).is_some() {
            // remove session from all games
            for (game_id, sessions) in self.games.iter_mut() {
                if sessions.remove(&msg.id) {
                    let _ = self.updates.send(Notification::UserDisconnected(*game_id));
                }
            }
        }
    }
}
