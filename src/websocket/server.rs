use actix::prelude::*;
use rand::{self, rngs::ThreadRng, Rng};
use std::collections::{HashMap, HashSet};

#[derive(Message)]
#[rtype(result = "()")]
pub struct Message(pub String);

#[derive(Message)]
#[rtype(usize)]
pub struct Connect {
    pub addr: Recipient<Message>,
    pub game_id: i64,
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct Disconnect {
    pub id: usize,
}

/// `TransactionServer` manages price updates/new sales
pub struct TransactionServer {
    sessions: HashMap<usize, Recipient<Message>>,
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
    pub fn notify_players(&self, game_id: i64) {
        if let Some(sessions) = self.games.get(&game_id) {
            for id in sessions {
                if let Some(addr) = self.sessions.get(id) {
                    let _ = addr.do_send(Message("A sale has happened".to_owned()));
                }
            }
        }
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

impl Handler<Transaction> for TransactionServer {
    type Result = ();

    fn handle(&mut self, tx: Transaction, _: &mut Context<Self>) {
        self.notify_players(tx.game_id);
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
