use std::collections::{HashMap, HashSet};
use std::sync::{mpsc, Arc};
use std::thread;

use actix::prelude::*;
use rand::{self, rngs::ThreadRng, Rng};

#[derive(Message)]
#[rtype(usize)]
pub struct Connect {
    pub addr: Recipient<Sale>,
    pub game_id: i64,
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct Disconnect {
    pub id: usize,
}

/// `TransactionServer` manages price updates/new sales
pub struct TransactionServer {
    sessions: HashMap<usize, Recipient<Sale>>,
    games: HashMap<i64, HashSet<usize>>,
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
    /// Send a message to a room, notifying them about a sale
    pub fn notify_players(&self, sale: Sale) {
        if let Some(sessions) = self.games.get(&sale.game_id) {
            for id in sessions {
                if let Some(addr) = self.sessions.get(id) {
                    let _ = addr.do_send(sale.clone());
                }
            }
        }
    }

    /// Listener receives sales updates and sends price updates to the clients
    pub fn listener(server: Arc<Addr<TransactionServer>>, rx: mpsc::Receiver<Sale>) {
        thread::spawn(move || {
            for received in rx {
                server.do_send(Sale {
                    game_id: received.game_id,
                    offsets: received.offsets,
                });
            }
        });
    }
}

/// Make actor from `ChatServer`
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

#[derive(Message, Debug)]
#[rtype(result = "()")]
pub struct Transaction {
    pub game_id: i64,
}

#[derive(Message, Debug, Serialize, Clone)]
#[rtype(result = "()")]
pub struct Sale {
    pub game_id: i64,
    pub offsets: HashMap<i16, i64>,
}

impl Handler<Sale> for TransactionServer {
    type Result = ();

    fn handle(&mut self, sale: Sale, _: &mut Context<Self>) {
        self.notify_players(sale);
    }
}

/// Handler for Disconnect message.
impl Handler<Disconnect> for TransactionServer {
    type Result = ();

    fn handle(&mut self, msg: Disconnect, _: &mut Context<Self>) {
        let mut games: Vec<i64> = Vec::new();

        // remove address
        if self.sessions.remove(&msg.id).is_some() {
            // remove session from all rooms
            for (name, sessions) in &mut self.games {
                if sessions.remove(&msg.id) {
                    games.push(name.to_owned());
                }
            }
        }
    }
}
