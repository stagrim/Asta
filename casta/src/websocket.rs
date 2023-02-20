use std::{fmt::Debug, time::{Instant, Duration}};

use actix::{Actor, StreamHandler, Message, Handler, AsyncContext};
use actix_web_actors::ws;
use serde::Serialize;

use crate::send_cached_to_view;

const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);
const CLIENT_TIMEOUT: Duration = Duration::from_secs(10);

pub struct Websocket {
    heart_beat: Instant
}

impl Websocket {
    pub fn new() -> Self {
        Websocket { 
            heart_beat: Instant::now(),
        }
    }

    fn heart_beat(&self, ctx: &mut ws::WebsocketContext<Self>) {
        ctx.run_interval(HEARTBEAT_INTERVAL, |act, ctx| {
            // Check if timeout has been reached for client heart beat
            // Since handle is not dropped, will try to disconnect until closing or replacing handle with new connection, since only one handle is kept
            if Instant::now().duration_since(act.heart_beat) > CLIENT_TIMEOUT {
                println!("Client heartbeat timeout reached, disconnecting");
                ctx.close(None);
                return;
            }
            // Ping client with empty binary message
            ctx.ping(b"");
        });
    }
}

impl Actor for Websocket {
    type Context = ws::WebsocketContext<Self>;
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for Websocket {
    // Unused since view don't communicate back, which this handle handles
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        let msg = match msg {
            Err(e) => {
                println!("Something went wrong: {e:?}");
                ctx.close(None);
                return;
            }
            Ok(msg) => msg,
        };

        match msg {
            ws::Message::Ping(msg) => {
                self.heart_beat = Instant::now();
                ctx.pong(&msg);
            }
            // Update last heart beat on pong received from client
            ws::Message::Pong(_) => self.heart_beat = Instant::now(),
            ws::Message::Text(text) => {
                println!("{}\nAddress{:?}", text, ctx.address());
                ctx.text(text)},
            ws::Message::Binary(bin) => {
                println!("{:?}\nAddress{:?}", bin, ctx.address());
                ctx.binary(bin)},
            _ => (),
        }
    }

    fn started(&mut self, ctx: &mut Self::Context) {
        self.heart_beat(ctx);
        println!("New Client detected");
        //This is dumb, find better method of calling async method in sync block
        tokio::task::spawn(async {
            send_cached_to_view().await;
        });
    }
    fn finished(&mut self, _ctx: &mut Self::Context) {
        println!("Client disconnected");
    }
}


#[derive(Message, Serialize)]
#[rtype(result = "()")]
pub struct Payload<T> {
    pub(crate) payload: T
}
 
impl<T> Handler<Payload<T>> for Websocket where T: Serialize + Debug {
    type Result = ();
 
    fn handle(&mut self, msg: Payload<T>, ctx: &mut Self::Context) {
        ctx.text(serde_json::to_string(&msg.payload).expect("Cannot serialize"));
    }
}
