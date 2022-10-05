mod websocket;

use std::{sync::Mutex, fmt::Debug};

use actix::Addr;
use actix_web::{web, App, HttpRequest, HttpResponse, Responder, HttpServer, get, post};
use actix_web_actors::ws::WsResponseBuilder;
use actix_files::NamedFile;
use serde::{Deserialize, Serialize};
use dotenv::dotenv;
use std::env;

struct AppState {
    handle: Mutex<Option<Addr<websocket::Websocket>>>
}

#[get("/")]
async fn index() -> impl Responder {
    NamedFile::open_async("./static/index.html").await.unwrap()
}

#[get("/index.js")]
async fn js() -> impl Responder {
    NamedFile::open_async("./static/target/index.js").await.unwrap()
}

//Replace with enum?
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
async fn set_url(data: web::Data<AppState>, api: web::Json<Api>) -> String {
    println!("api: {:?}", api);
    let handle = data.handle.lock().unwrap();
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
async fn get_ws(req: HttpRequest, stream: web::Payload, data: web::Data<AppState>) -> HttpResponse {
    let mut handle = data.handle.lock().unwrap();
    let (addr, resp) = WsResponseBuilder::new(websocket::Websocket, &req, stream).start_with_addr().expect("An error occurred during the handshake");
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
        .service(js)
        .service(set_url)
        .service(get_ws)
    ).bind(format!("0.0.0.0:{port}"))?.run().await
}
