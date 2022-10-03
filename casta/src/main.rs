use std::{sync::Mutex, fmt::Debug};

use actix::{Actor, StreamHandler, AsyncContext, Addr, Message, Handler};
use actix_web::{web, App, HttpRequest, HttpResponse, Responder, HttpServer, get};
use actix_web_actors::ws::{self, WsResponseBuilder};
use actix_files::NamedFile;
use serde::{Serialize, Deserialize};
use dotenv::dotenv;
use std::env;

struct AppState {
    handle: Mutex<Option<Addr<Websocket>>>
}

#[derive(Deserialize, Debug)]
struct ApiRequest {
    url: String
}
#[derive(Deserialize, Debug)]
struct YoutubeRequest {
    id: String
}

struct Websocket;

impl Actor for Websocket {
    type Context = ws::WebsocketContext<Self>;
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for Websocket {
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
    }
    fn finished(&mut self, _ctx: &mut Self::Context) {
        println!("Client disconnected");
    }
}


#[derive(Message)]
#[rtype(result = "()")]
pub struct Payload<T> {
    pub payload: T,
}
 
impl<T> Handler<Payload<T>> for Websocket where T: Serialize + Debug {
    type Result = ();
 
    fn handle(&mut self, msg: Payload<T>, ctx: &mut Self::Context) {
        ctx.text(serde_json::to_string(&msg.payload).expect("Cannot serialize"));
    }
}

#[get("/")]
async fn index() -> impl Responder {
    NamedFile::open_async("./static/index.html").await.unwrap()
}

//TODO: add response with errors, like no connected clients
#[get("/api")]
async fn set_url(data: web::Data<AppState>, queries: web::Query<ApiRequest>) -> String {
    println!("url: {:?}", queries);
    let handle = data.handle.lock().unwrap();
    let h = handle.clone().unwrap();
    let r = h.recipient().send(Payload { payload: queries.url.clone() }).await;
    format!("r: {:?}", r)
}

//TODO: redirect to /api instead
#[get("/youtube")]
async fn youtube(data: web::Data<AppState>, queries: web::Query<YoutubeRequest>) -> String {
    println!("url: {:?}", queries);
    let handle = data.handle.lock().unwrap();
    let h = handle.clone().unwrap();
    let r = h.recipient().send(Payload { payload: format!("https://www.youtube.com/embed/{}?autoplay=1&controls=0&rel=0&modestbranding=1", queries.id) }).await;
    format!("r: {:?}", r)
}

#[get("/ws")]
async fn websocket(req: HttpRequest, stream: web::Payload, data: web::Data<AppState>) -> HttpResponse {
    let mut handle = data.handle.lock().unwrap();
    let (addr, resp) = WsResponseBuilder::new(Websocket, &req, stream).start_with_addr().unwrap();
    *handle = Some(addr);
    resp
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();

    let port = env::var("PORT").unwrap_or("3000".to_string());

    let app_state = web::Data::new(AppState {
        handle: Mutex::new(None)
    });

    HttpServer::new(move || App::new()
        .app_data(app_state.clone())
        .service(index)
        .service(set_url)
        .service(youtube)
        .service(websocket)
    ).bind(format!("127.0.0.1:{port}"))?.run().await
}
