use std::fmt::Debug;

use actix::{Actor, StreamHandler, Message, Handler, AsyncContext};
use actix_web_actors::ws;
use serde::Serialize;

use crate::send_cached_to_view;

pub struct Websocket;

impl Actor for Websocket {
    type Context = ws::WebsocketContext<Self>;
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for Websocket {
    // Unused since view don't communicate back, which this handle handles
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        match msg {
            Ok(ws::Message::Ping(msg)) => {
                println!("{:?}\nAddress{:?}", msg, ctx.address());
                ctx.pong(&msg)
            },
            Ok(ws::Message::Text(text)) => {
                println!("{}\nAddress{:?}", text, ctx.address());
                ctx.text(text)},
            Ok(ws::Message::Binary(bin)) => {
                println!("{:?}\nAddress{:?}", bin, ctx.address());
                ctx.binary(bin)},
            _ => (),
        }
    }
    fn started(&mut self, _ctx: &mut Self::Context) {
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
