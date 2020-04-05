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

/// Send message to specific room
#[derive(Message, Debug)]
#[rtype(result = "()")]
pub struct ClientMessage {
    /// Id of the client session
    pub id: usize,
    /// Peer message
    pub msg: String,
    /// Room name
    pub game_id: i64,
}

/// Join room, if room does not exists create new one.
#[derive(Message)]
#[rtype(result = "()")]
pub struct Join {
    /// Client id
    pub id: usize,
    /// Game ID
    pub game_id: i64,
}

/// `ChatServer` manages chat rooms and responsible for coordinating chat
/// session. implementation is super primitive
pub struct ChatServer {
    sessions: HashMap<usize, Recipient<Message>>,
    games: HashMap<i64, HashSet<usize>>,
    rng: ThreadRng,
}

impl Default for ChatServer {
    fn default() -> ChatServer {
        let mut games = HashMap::new();

        // TODO: automatically create the game channel if it doesn't exist
        games.insert(8, HashSet::new());

        ChatServer {
            sessions: HashMap::new(),
            games,
            rng: rand::thread_rng(),
        }
    }
}

impl ChatServer {
    /// Send message to all users in the room
    pub fn send_message(&self, game_id: &i64, message: &str, skip_id: usize) {
        if let Some(sessions) = self.games.get(game_id) {
            for id in sessions {
                if *id != skip_id {
                    if let Some(addr) = self.sessions.get(id) {
                        let _ = addr.do_send(Message(message.to_owned()));
                    }
                }
            }
        }
    }

    /// Send a message to a room, notifying them about a sale
    pub fn notify_players(&self, game_id: &i64) {
        if let Some(sessions) = self.games.get(game_id) {
            for id in sessions {
                if let Some(addr) = self.sessions.get(id) {
                    let _ = addr.do_send(Message("A sale has happened".to_owned()));
                }
            }
        }
    }
}

/// Make actor from `ChatServer`
impl Actor for ChatServer {
    /// We are going to use simple Context, we just need ability to communicate
    /// with other actors.
    type Context = Context<Self>;
}

/// Handler for Connect message.
///
/// Register new session and assign unique id to this session
impl Handler<Connect> for ChatServer {
    type Result = usize;

    fn handle(&mut self, msg: Connect, _: &mut Context<Self>) -> Self::Result {
        // notify all users in same room
        self.send_message(&msg.game_id, "Someone joined", 0);

        // register session with random id
        let id = self.rng.gen::<usize>();
        self.sessions.insert(id, msg.addr);

        // join the games' notification channel
        self.games.get_mut(&msg.game_id).unwrap().insert(id);

        // send id back
        id
    }
}

#[derive(Message, Debug)]
#[rtype(result = "()")]
pub struct Transaction {
    /// Id of the client session
    pub game_id: i64,
}

impl Handler<Transaction> for ChatServer {
    type Result = ();

    fn handle(&mut self, tx: Transaction, _: &mut Context<Self>) {
        self.notify_players(&tx.game_id);
    }
}

/// Handler for Disconnect message.
impl Handler<Disconnect> for ChatServer {
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
        // send message to other users
        for game in games {
            self.send_message(&game, "Someone disconnected", 0);
        }
    }
}

/// Handler for Message message.
impl Handler<ClientMessage> for ChatServer {
    type Result = ();

    fn handle(&mut self, msg: ClientMessage, _: &mut Context<Self>) {
        debug!("ClientMessage: {:?}", msg);
        self.send_message(&msg.game_id, msg.msg.as_str(), msg.id);
    }
}

/// Join room, send disconnect message to old room
/// send join message to new room
impl Handler<Join> for ChatServer {
    type Result = ();

    fn handle(&mut self, msg: Join, _: &mut Context<Self>) {
        debug!("in join handler");
        let Join { id, game_id } = msg;
        let mut games = Vec::new();

        // remove session from all rooms
        for (n, sessions) in &mut self.games {
            if sessions.remove(&id) {
                games.push(n.to_owned());
            }
        }
        // send message to other users
        for game in games {
            self.send_message(&game, "Someone disconnected", 0);
        }

        if self.games.get_mut(&game_id).is_none() {
            self.games.insert(game_id.clone(), HashSet::new());
        }
        self.send_message(&game_id, "Someone connected", id);
        self.games.get_mut(&game_id).unwrap().insert(id);
    }
}
