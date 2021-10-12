use std::time::{Duration, Instant};

use actix::prelude::*;
use actix_identity::Identity;
use actix_web::web::{Data, Path};
use actix_web::{web, HttpRequest};
use actix_web_actors::ws;

use crate::auth;
use crate::games::Game;
use crate::server::State;
use crate::users::User;
use crate::websocket::server;
use crate::websocket::server::{ConnectionType, GameId};

/// How often heartbeat pings are sent
const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);
/// How long before lack of client response causes a timeout
const CLIENT_TIMEOUT: Duration = Duration::from_secs(10);

/// route used for game updates
pub async fn game_route(
    req: HttpRequest,
    stream: web::Payload,
    game_id: Path<i64>,
    id: Identity,
    state: Data<State>,
) -> crate::server::Response {
    let mut user = auth::get_user(&id)?;

    if !Game::verify_user_participation(*game_id, user.id, &state.db).await? && !user.is_admin {
        forbidden!("you are not in this game");
    }

    // When an administrator connects to this route, his admin flag is set to false so he only receives normal user notifications
    // This is a temporary workaround and should be changed once the server knows if a connected user wants game updates or admin updates
    user.is_admin = false;

    ws::start(
        WebsocketConnection {
            id: 0,
            hb: Instant::now(),
            connection_type: ConnectionType::GameConnection(GameId(*game_id)),
            user,
            notifier: state.notifier.clone(),
        },
        &req,
        stream,
    )
    .map_err(|e| e.into())
}

/// Entry point for our route
pub async fn admin_route(
    req: HttpRequest,
    stream: web::Payload,
    state: Data<State>,
    id: Identity,
) -> crate::server::Response {
    let user = auth::get_user(&id)?;

    if !user.is_admin {
        forbidden!("insufficient permissions");
    }

    ws::start(
        WebsocketConnection {
            id: 0,
            hb: Instant::now(),
            connection_type: ConnectionType::AdminConnection,
            user,
            notifier: state.notifier.clone(),
        },
        &req,
        stream,
    )
    .map_err(|e| e.into())
}

struct WebsocketConnection {
    /// unique session id
    /// Get's filled in when connecting
    id: usize,
    /// Client must send ping at least once per 10 seconds (CLIENT_TIMEOUT),
    /// otherwise we drop connection.
    hb: Instant,
    /// joined game
    connection_type: ConnectionType,
    /// Connected user
    user: User,
    /// notification server
    notifier: Addr<server::NotificationServer>,
}

impl Actor for WebsocketConnection {
    type Context = ws::WebsocketContext<Self>;

    /// Method is called on actor start.
    /// We register ws session with NotificationServer
    fn started(&mut self, ctx: &mut Self::Context) {
        // we'll start heartbeat process on session start.
        self.hb(ctx);

        // register self in notification server. `AsyncContext::wait` register
        // future within context, but context waits until this future resolves
        // before processing any other events.
        // HttpContext::state() is instance of WebsocketConnection, state is shared
        // across all routes within application
        let addr = ctx.address();
        self.notifier
            .send(server::Connect {
                addr: addr.recipient(),
                user: self.user.clone(),
                connection_type: self.connection_type,
            })
            .into_actor(self)
            .then(|res, act, ctx| {
                match res {
                    Ok(res) => act.id = res,
                    // something is wrong with notification server
                    Err(e) => {
                        error!("unable to start websocket connection: {}", e);
                        ctx.stop();
                    }
                }
                fut::ready(())
            })
            .wait(ctx);
        debug!("{} connected to the websocket", self.user.username);
    }

    fn stopping(&mut self, _: &mut Self::Context) -> Running {
        self.notifier.do_send(server::Disconnect { id: self.id });
        Running::Stop
    }
}

/// Handle messages from server, we simply send it to peer websocket
impl Handler<server::Notification> for WebsocketConnection {
    type Result = ();

    fn handle(&mut self, notification: server::Notification, ctx: &mut Self::Context) {
        let json = match serde_json::to_string(&notification) {
            Ok(json) => json,
            Err(error) => {
                // This should never happen
                error!(
                    "unable to serialize websocket message: {:?}, error: {}",
                    notification, error
                );
                return;
            }
        };
        ctx.text(json);
    }
}

/// WebSocket message handler
impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for WebsocketConnection {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        let msg = match msg {
            Err(error) => {
                error!("Invalid websocket protocol message: {}", error);
                ctx.close(Some(ws::CloseReason::from(ws::CloseCode::Protocol)));
                ctx.stop();
                return;
            }
            Ok(msg) => msg,
        };

        trace!("Websocket received message: {:?}", msg);
        match msg {
            ws::Message::Ping(msg) => {
                self.hb = Instant::now();
                ctx.pong(&msg);
            }
            ws::Message::Pong(_) => {
                self.hb = Instant::now();
            }
            ws::Message::Text(_) => {
                debug!("ignoring incoming messages for now");
                ctx.close(Some(ws::CloseReason::from(ws::CloseCode::Unsupported)));
                ctx.stop();
            }
            ws::Message::Binary(_) => {
                debug!("Unexpected binary");
                ctx.close(Some(ws::CloseReason::from(ws::CloseCode::Unsupported)));
                ctx.stop();
            }
            ws::Message::Close(reason) => {
                ctx.close(reason);
                ctx.stop();
            }
            ws::Message::Continuation(_) => {
                ctx.stop();
            }
            ws::Message::Nop => (),
        }
    }
}

impl WebsocketConnection {
    /// helper method that sends ping to client every second.
    ///
    /// also this method checks heartbeats from client
    fn hb(&self, ctx: &mut ws::WebsocketContext<Self>) {
        ctx.run_interval(HEARTBEAT_INTERVAL, |act, ctx| {
            // check client heartbeats
            if Instant::now().duration_since(act.hb) > CLIENT_TIMEOUT {
                // heartbeat timed out
                error!("Websocket Client heartbeat failed, disconnecting!");

                // notify server
                act.notifier.do_send(server::Disconnect { id: act.id });

                // stop actor
                ctx.stop();

                // don't try to send a ping
                return;
            }

            ctx.ping(b"");
        });
    }
}
