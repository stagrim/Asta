mod websocket;
mod sasta;

use std::fs::File;
use std::io::BufReader;
use std::process;
use std::sync::Arc;
use std::{fmt::Debug, env};

use actix::Addr;
use actix_web::{web, App, HttpRequest, HttpResponse, HttpServer, get};
use actix_web_actors::ws::WsResponseBuilder;
use actix_files::Files;
use adler::adler32;
use sasta::{Sasta, SastaResponse, DisplayData};
use tokio::signal;
use serde::Serialize;
use dotenv::dotenv;
use lazy_static::lazy_static;
use tokio::sync::Mutex;

use crate::sasta::WebsiteData;

//Other solutions not including a shared mutable state are welcome
lazy_static! {
    static ref HANDLE: Mutex<Option<Addr<websocket::Websocket>>> = Mutex::new(None);
}

#[derive(Serialize, Debug, Clone)]
pub enum ClientPayload {
    Display(DisplayData),
    Disconnected(),
    Hash(String)
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
            websocket::Websocket::new(cached_display_req.into_inner().clone(), *req.app_data::<u32>().unwrap()),
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

    let file = File::open("./static/target/index.js")
        .expect("Error frontend does not seem to be built, run \"npm run build\"");
    let mut file = BufReader::new(file);

    let hash = adler32(&mut file).expect("Could not calculate hash");

    println!("Starting Casta\n");
    println!("Serving Casta on http://0.0.0.0:{casta_port}");
    println!("Connecting to Sasta on ws://{address}:{port}");
    println!("hostname={hostname}");
    println!("uuid={}\n", &uuid);

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
        let mut sasta = Sasta::new(address, port, uuid.clone(), hostname).await;
        
        loop {
            match sasta.read_message().await {
                Some(resp) => {
                    let mut api: Option<ClientPayload> = None;
                    match resp {
                        SastaResponse::Name(name) => println!("Handshake done, received name {name:?}"),
                        SastaResponse::Pending(pending) => {
                            println!("[Pending {}] Waiting to be defined in Sasta", pending);
                            let send = ClientPayload::Display(DisplayData::Text { 
                                data: WebsiteData {
                                    content: format!("Pending connection to Sasta with uuid {}", &uuid)
                                },
                            });
                            send_to_view(send.clone()).await;
                            api = Some(send);
                        },
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
        .app_data(hash)
        .app_data(web::Data::from(cached_display_req_2.clone()))
        .service(get_ws)
        .service(Files::new("/", "./static").index_file("index.html"))
    ).bind(format!("0.0.0.0:{casta_port}"))?.run().await
}
