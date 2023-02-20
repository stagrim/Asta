mod websocket;
mod sasta;

use std::process;
use std::sync::Arc;
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

// TODO: View page detailing pending connection from Sasta and show UUID

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

pub async fn send_cached_to_view(cached_display_req: Arc<Mutex<Option<ClientPayload>>>) {
    let cached = cached_display_req.lock().await;
    match cached.clone() {
        Some(c) => { send_to_view(c).await; },
        None => (),
    }
}

///Returns a websocket connection
#[get("/ws")]
async fn get_ws(cached_display_req: web::Data<Mutex<Option<ClientPayload>>>, req: HttpRequest, stream: web::Payload) -> HttpResponse {

    let (addr, resp) =
        WsResponseBuilder::new(
            websocket::Websocket::new(cached_display_req.into_inner().clone()),
            &req,
            stream,
        ).start_with_addr()
        .expect("An error occurred during the handshake");
    
    *HANDLE.lock().await = Some(addr);
    resp
}

async fn send_disconnected_to_view(cached_display_req: Arc<Mutex<Option<ClientPayload>>>) {
    let api = ClientPayload::Disconnected();
    send_to_view(api.clone()).await;
    *cached_display_req.lock().await = Some(api);
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();
    let uuid = env::var("UUID").expect("Must provide a UUID");
    let port = env::var("PORT").expect("Must provide a port");
    let casta_port = env::var("CASTA_PORT").unwrap_or("3000".to_string());
    let address = env::var("ADDRESS").expect("Must provide an address");
    let hostname = env::var("HOSTNAME").unwrap_or("Casta Client".to_string());

    println!("Starting Casta\n");
    println!("Serving Casta on http://0.0.0.0:{casta_port}");
    println!("Connecting to Sasta on ws://{address}:{port}");
    println!("hostname={hostname}");
    println!("uuid={uuid}\n");

    let cached_display_req = Arc::new(Mutex::new(None::<ClientPayload>));
    let cached_display_req_2 = cached_display_req.clone();

    tokio::task::spawn(async {
        match signal::ctrl_c().await {
            Ok(_) => println!("\nReceived shutdown signal, exiting..."),
            Err(err) => {
                eprintln!("Unable to listen for shutdown signal: {err}");
            },
        }
        process::exit(0);
    });

    tokio::task::spawn(async move {
        send_disconnected_to_view(cached_display_req.clone()).await;
        let mut sasta = Sasta::new(address, port, uuid, hostname).await;
        
        loop {
            match sasta.read_message().await {
                Some(resp) => {
                    let mut api: Option<ClientPayload> = None;
                    match resp {
                        SastaResponse::Name(name) => println!("Handshake done, received name {name:?}"),
                        SastaResponse::Display(display) => {
                            println!("[Message] {:?}", display);
                            let send = ClientPayload::Display(display);
                            send_to_view(send.clone()).await;
                            api = Some(send);
                        }
                    }

                    // Updates mutex cache with new latest display request form Sasta
                    if let Some(a) = api {
                        *cached_display_req.lock().await = Some(a);
                    }
                },
                None => {
                    send_disconnected_to_view(cached_display_req.clone()).await;
                    sasta.reconnect().await
                }
            }
        }
    });

    HttpServer::new(move || App::new()
        .app_data(web::Data::from(cached_display_req_2.clone()))
        .service(index)
        .service(js)
        .service(disconnected_image)
        .service(get_ws)
    ).bind(format!("0.0.0.0:{casta_port}"))?.run().await
}
