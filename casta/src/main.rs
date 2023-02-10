mod websocket;
mod sasta;

use std::process;
use std::{fmt::Debug, env};

use actix::Addr;
use actix_web::{web, App, HttpRequest, HttpResponse, Responder, HttpServer, get};
use actix_web_actors::ws::WsResponseBuilder;
use actix_files::NamedFile;
use sasta::{Sasta, SastaResponse, DisplayData};
use tokio::signal;
use serde::Serialize;
use dotenv::dotenv;
use lazy_static::lazy_static;
use tokio::sync::Mutex;

// TODO: Set these during Actix build call in main instead?
#[get("/")]
async fn index() -> impl Responder {
    NamedFile::open_async("./static/index.html").await.unwrap()
}

#[get("/index.js")]
async fn js() -> impl Responder {
    NamedFile::open_async("./static/target/index.js").await.unwrap()
}

#[get("/disconnected")]
async fn disconnected_image() -> impl Responder {
    NamedFile::open_async("./static/disconnect.png").await.unwrap()
}

//Other solutions not including a shared mutable state are welcome
lazy_static! {
    static ref HANDLE: Mutex<Option<Addr<websocket::Websocket>>> = Mutex::new(None);
    static ref CACHED: Mutex<Option<ClientPayload>> = Mutex::new(None);
}

#[derive(Serialize, Debug, Clone)]
pub enum ClientPayload {
    Display(DisplayData),
    Disconnected()
}

async fn send_to_view(payload: ClientPayload) -> String {
    let handle = HANDLE.lock().await;
    match handle.clone() {
        Some(h) => {
            let r = h.recipient().send(websocket::Payload { payload }).await;
            format!("result: {:?}", r)
        },
        None => format!("Handle has not yet been created. Let a client connect to the websocket and try again")
    }
}

pub async fn send_cached_to_view() {
    let cached = CACHED.lock().await;
    match cached.clone() {
        Some(c) => { send_to_view(c).await; },
        None => (),
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

async fn handle_sasta_response(response: SastaResponse) {
    let mut api: Option<ClientPayload> = None;
    match response {
        SastaResponse::Name(name) => println!("Handshake done, received name \"{name}\""),
        SastaResponse::Display(display) => {
            println!("[Message] {:?}", display);
            let send = ClientPayload::Display(display);
            send_to_view(send.clone()).await;
            api = Some(send);
        }
    }
    if let Some(a) = api {
        cache_api(a).await;
    }
}

/// Caches api call given to send to newly connected devices
async fn cache_api(api: ClientPayload) {
    let mut cached = CACHED.lock().await;
    *cached = Some(api);
}

async fn send_disconnected_to_view() {
    let api = ClientPayload::Disconnected();
    send_to_view(api.clone()).await;
    cache_api(api).await;
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();
    let uuid = env::var("UUID").expect("Must provide a UUID");
    let port = env::var("PORT").expect("Must provide a port");
    let casta_port = env::var("CASTA_PORT").unwrap_or("3000".to_string());
    let address = env::var("ADDRESS").expect("Must provide an address");
    let hostname = env::var("HOSTNAME").unwrap_or("Casta Client".to_string());

    println!("Started Casta\n");
    println!("Serving Casta on http://127.0.0.1/{casta_port}");
    println!("Connecting to Sasta on ws://{address}/{port}");
    println!("hostname={hostname}");
    println!("uuid={uuid}\n");

    tokio::task::spawn(async {
        match signal::ctrl_c().await {
            Ok(()) => println!("\nReceived shutdown signal, exiting..."),
            Err(err) => {
                eprintln!("Unable to listen for shutdown signal: {}", err);
            },
        }
        process::exit(0);
    });

    tokio::task::spawn(async {
        send_disconnected_to_view().await;
        let mut sasta = Sasta::new(address, port, uuid, hostname).await;
        
        loop {
            match sasta.read_message().await {
                Some(resp) => handle_sasta_response(resp).await,
                None => {
                    send_disconnected_to_view().await;
                    sasta.reconnect().await
                }
            }
        }
    });

    HttpServer::new(move || App::new()
        .service(index)
        .service(js)
        .service(disconnected_image)
        .service(get_ws)
    ).bind(format!("0.0.0.0:{casta_port}"))?.run().await
}
