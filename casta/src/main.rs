mod websocket;
mod sasta;

use std::process;
use std::{fmt::Debug, env};

use actix::Addr;
use actix_web::{web, App, HttpRequest, HttpResponse, Responder, HttpServer, get, post};
use actix_web_actors::ws::WsResponseBuilder;
use actix_files::NamedFile;
use sasta::{Sasta, SastaResponse, DisplayData};
use tokio::signal;
use serde::{Deserialize, Serialize};
use dotenv::dotenv;
use lazy_static::lazy_static;
use tokio::sync::Mutex;

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
    static ref CACHED: Mutex<Option<Api>> = Mutex::new(None);
}

//TODO: old Api was created before actually understanding how Sasta would work, so currently the Sasta requests are converted to this old Api. fix!
#[derive(Serialize, Deserialize, Default, Debug, Clone)]
struct Api {
    #[serde(rename(deserialize = "WEBSITE"))]
    website: Option<String>,
    #[serde(rename(deserialize = "IMAGE"))]
    image: Option<String>,
    #[serde(rename(deserialize = "VIDEO"))]
    video: Option<String>,
    //TODO: replace with image, background_audio server side?
    #[serde(rename(deserialize = "AUDIO"))]
    audio: Option<String>,
    #[serde(rename(deserialize = "BACKGROUND_AUDIO"))]
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

pub async fn send_cached_to_view() {
    let cached = CACHED.lock().await;
    match cached.clone() {
        Some(c) => { send_to_view(c).await; },
        None => (),
    }
}

//TODO?: If websocket disconnects, no data will be sent to it since last SastaResponse is not cached. Send cached result to newly connected frontend?
///Returns a websocket connection
#[get("/ws")]
async fn get_ws(req: HttpRequest, stream: web::Payload) -> HttpResponse {
    let mut handle = HANDLE.lock().await;

    let (addr, resp) = WsResponseBuilder::new(websocket::Websocket, &req, stream).start_with_addr().expect("An error occurred during the handshake");
    *handle = Some(addr);
    resp
}

async fn handle_sasta_response(response: SastaResponse) {
    let mut api = Api::default();
    match response {
        SastaResponse::Name(name) => println!("Handshake done, received name \"{name}\""),
        SastaResponse::Display(display) => {
            match display {
                DisplayData::Website { data } => {
                    println!("[Message] Website {:?}", data.content);
                    api.website = Some(data.content);
                    send_to_view(api.clone()).await;
                },
            }
        }
    }
    let mut cached = CACHED.lock().await;
    *cached = Some(api);
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
        let mut sasta = Sasta::new(address, port, uuid, hostname).await;

        loop {
            let resp: SastaResponse = sasta.read_message().await;
            handle_sasta_response(resp).await;
        }
    });

    HttpServer::new(move || App::new()
        .service(index)
        .service(js)
        .service(set_url)
        .service(get_ws)
    ).bind(format!("127.0.0.1:{casta_port}"))?.run().await
}
