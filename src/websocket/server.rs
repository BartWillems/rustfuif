use std::collections::{HashMap, HashSet};
use std::sync::{mpsc, Arc};
use std::thread;

use actix::prelude::*;
use rand::{self, rngs::ThreadRng, Rng};

use crate::transactions::Transaction;

#[derive(Message)]
#[rtype(usize)]
pub struct Connect {
    pub addr: Recipient<Notification>,
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

type GameId = i64;
type SessionId = usize;

/// `TransactionServer` manages price updates/new sales
pub struct TransactionServer {
    sessions: HashMap<SessionId, Recipient<Notification>>,
    games: HashMap<GameId, HashSet<SessionId>>,
    rng: ThreadRng,
}

impl Default for TransactionServer {
    fn default() -> TransactionServer {
        TransactionServer {
            sessions: HashMap::new(),
            games: HashMap::new(),
            rng: rand::thread_rng(),
        }
    }
}

impl TransactionServer {
    /// Notify all players of a game that a sale happened
    pub fn notify_sale(&self, sale: Sale) {
        if let Some(sessions) = self.games.get(&sale.game_id) {
            for id in sessions {
                if let Some(addr) = self.sessions.get(id) {
                    let _ = addr.do_send(Notification::NewSale(sale.clone()));
                }
            }
        }
    }

    pub fn notify_price_update(&self) {
        for (_, recipient) in self.sessions.iter() {
            let _ = recipient.do_send(Notification::PriceUpdate);
        }
    }

    /// Listener receives sales updates and sends price updates to the clients
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
        self.sessions.insert(id, msg.addr);

        self.games
            .entry(msg.game_id)
            .or_insert_with(HashSet::new)
            .insert(id);

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

#[derive(Message, Debug, Serialize, Clone)]
#[rtype(result = "()")]
pub enum Notification {
    NewSale(Sale),
    PriceUpdate,
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
            for sessions in self.games.values_mut() {
                sessions.remove(&msg.id);
            }
        }
    }
}
