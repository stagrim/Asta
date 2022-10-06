mod websocket;

use std::{fmt::Debug, env};

use actix::Addr;
use actix_web::{web, App, HttpRequest, HttpResponse, Responder, HttpServer, get, post};
use actix_web_actors::ws::WsResponseBuilder;
use actix_files::NamedFile;
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
        //TODO: Websocket to sasta goes here
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(2));
        loop {
            interval.tick().await;
            let api = Api {
                website: Some("https://dsek.se/".to_string()),
                image: None,
                video: None,
                audio: None,
                background_audio: None,
            };
            println!("spawned thread: {}", send_to_view(api).await);
        };
    });

    HttpServer::new(move || App::new()
        .service(index)
        .service(js)
        .service(set_url)
        .service(get_ws)
    ).bind(format!("0.0.0.0:{port}"))?.run().await
}
