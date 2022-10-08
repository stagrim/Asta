mod websocket;
mod sasta;

use std::{fmt::Debug, env};

use actix::Addr;
use actix_web::{web, App, HttpRequest, HttpResponse, Responder, HttpServer, get, post};
use actix_web_actors::ws::WsResponseBuilder;
use actix_files::NamedFile;
use tokio_tungstenite::tungstenite::error::ProtocolError;
use tokio_tungstenite::tungstenite::{self, Error};
use tokio_tungstenite::tungstenite::{connect, Message};
use serde::{Deserialize, Serialize};
use dotenv::dotenv;
use lazy_static::lazy_static;
use tokio::sync::Mutex;
use url::Url;

#[get("/")]
async fn index() -> impl Responder {
    NamedFile::open_async("./static/index.html").await.unwrap()
}

#[get("/index.js")]
async fn js() -> impl Responder {
    NamedFile::open_async("./static/target/index.js").await.unwrap()
}

//Other solutions not including a shared mutable state are welcome
lazy_static! {
    static ref HANDLE: Mutex<Option<Addr<websocket::Websocket>>> = Mutex::new(None);
}

#[derive(Serialize, Deserialize, Debug)]
struct Api {
    #[serde(rename(deserialize = "Website"))]
    website: Option<String>,
    #[serde(rename(deserialize = "Image"))]
    image: Option<String>,
    #[serde(rename(deserialize = "Video"))]
    video: Option<String>,
    //TODO: replace with image, background_audio server side?
    #[serde(rename(deserialize = "Audio"))]
    audio: Option<String>,
    #[serde(rename(deserialize = "Background_audio"))]
    background_audio: Option<String>,
}

/// Takes post request as struct Api and sends it to clients connected to websocket
#[post("/api")]
async fn set_url(api: web::Json<Api>) -> String {
    println!("api: {:?}", api);
    send_to_view(api.into_inner()).await
}

async fn send_to_view(api: Api) -> String {
    let handle = HANDLE.lock().await;
    match handle.clone() {
        Some(h) => {
            let r = h.recipient().send(websocket::Payload { payload: api }).await;
            format!("result: {:?}", r)
        },
        None => format!("Handle has not yet been created. Let a client connect to the websocket and try again")
    }
}

///Returns a websocket connection
#[get("/ws")]
async fn get_ws(req: HttpRequest, stream: web::Payload) -> HttpResponse {
    let mut handle = HANDLE.lock().await;

    let (addr, resp) = WsResponseBuilder::new(websocket::Websocket, &req, stream).start_with_addr().expect("An error occurred during the handshake");
    *handle = Some(addr);
    resp
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();

    let port = env::var("PORT").unwrap_or("3000".to_string());

    tokio::task::spawn(async {
        let (mut socket, response) = connect(Url::parse("ws://localhost:8000").unwrap()).expect("Can't connect");
        println!("Response on connect: {:?}", response);
        socket.write_message(Message::Text(r#"{
            "uuid": "47e7836b-ddfe-40f4-aaeb-e2db22bc1a24",
            "hostname": "Bernadetta"
        }"#.into())).unwrap();
        
        // Loop forever, handling parsing each message
        loop {

            match socket.read_message() {
                Ok(msg) => {
                    let msg = match msg {
                        tungstenite::Message::Text(s) => { s }
                        tungstenite::Message::Ping(s) => { 
                            format!("Received Ping from server with payload: [{}]", s.into_iter().map(|s| s.to_string()).collect::<String>())
                        }
                        _ => { println!("{:?}", msg); panic!() }
                    };
                    println!("{}", msg);
                    // let parsed: serde_json::Value = serde_json::from_str(&msg).expect("Can't parse to JSON");
                    // println!("{:?}", parsed["result"]);
                },
                Err(e) => { 
                    println!("Error: {}", e);
                    let close = socket.close(None);
                    println!("close: {:?}", close);
                    println!("Connection closed, attempting reconnect");
                    break;
                }
            }

            
        }
    });

    HttpServer::new(move || App::new()
        .service(index)
        .service(js)
        .service(set_url)
        .service(get_ws)
    ).bind(format!("0.0.0.0:{port}"))?.run().await
}
