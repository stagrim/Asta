use core::time;
use std::{net::SocketAddr, sync::Arc, time::Duration};

use axum::{Router, routing::get, extract::{WebSocketUpgrade, ConnectInfo, ws::{Message, WebSocket}, State}, response::IntoResponse};
use futures_util::{StreamExt, SinkExt, stream::SplitSink};
use serde::{Deserialize, Serialize};
use store::store::{Store, Content};
use tokio::{sync::{Mutex, watch::{self, Receiver}}};

mod store;

#[derive(Clone)]
struct ServerState {
    // content: Mutex<Content>,
    rx: Receiver<Content>
}

#[tokio::main]
async fn main() {
    let store = Store::new();

    
    let loaded = store.load().await;
    println!("{:#?}", loaded);

    let (tx, rx) = watch::channel(loaded);

    let server_state = ServerState { rx };

    let app = Router::new()
        .route("/", get(ws_handler))
        .with_state(server_state);

    // run it with hyper
    let addr = SocketAddr::from(([0, 0, 0, 0], 8040));
    println!("listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service_with_connect_info::<SocketAddr>())
        .await
        .unwrap();
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(state): State<ServerState>,
) -> impl IntoResponse {
    println!("{addr} connected.");
    // finalize the upgrade process by returning upgrade callback.
    // we can customize the callback by sending additional info such as address.
    ws.on_upgrade(move |socket| handle_socket(socket, addr, state))
}

#[derive(Deserialize)]
struct HelloRequest {
    uuid: String,
    hostname: String
}

#[derive(Debug, Serialize)]
pub enum Payload {
    #[serde(rename(serialize = "display"))]
    Display(DisplayPayload),
    #[serde(rename(serialize = "name"))]
    Name(String),
    #[serde(rename(serialize = "pending"))]
    Pending(bool)
}

#[derive(Serialize, Debug, Clone)]
#[serde(tag = "type")]
pub enum DisplayPayload {
    #[serde(rename(serialize = "WEBSITE"))]
    Website { data: WebsitePayload },
    #[serde(rename(serialize = "TEXT"))]
    Text { data: WebsitePayload },
    #[serde(rename(serialize = "IMAGE"))]
    Image { data: WebsitePayload }
}

#[derive(Serialize, Debug, Clone)]
pub struct WebsitePayload {
    pub content: String
}

async fn handle_socket(socket: WebSocket, who: SocketAddr, state: ServerState) {
    let (sender, mut reciever) = socket.split();
    let sender = Arc::new(Mutex::new(sender));
    let mut hb = heart_beat(sender.clone());

    let mut send = tokio::spawn(async move {

        let hello: HelloRequest = loop {
            if let Some(Ok(Message::Text(msg))) = reciever.next().await {
                match serde_json::from_str(&msg) {
                    Ok(msg) => break msg,
                    Err(_) => {
                        println!("{msg:?} was not a HelloRequest");
                        continue
                    },
                };
            }
        };
        let rx = state.rx.clone();
        let content = rx.borrow().clone();
        let display = loop {
            match content.displays.get(&hello.uuid) {
                Some(d) => break d,
                None => {
                    println!("No screen defined with UUID {:?}", hello.uuid);
                    sender.lock().await.send(Message::Text(r#"{"pending":true}"#.to_owned())).await.unwrap();
                    std::thread::sleep(Duration::from_secs(10));
                    continue;
                },
            }
        };

        let schedule = &display.schedule;
        let playlist = content.playlists.get(&content.schedules.get(schedule).unwrap().playlist).unwrap().items.clone();
        drop(content);
        
        if sender.lock().await.send(Message::Text(r#"{"name":"TEST"}"#.to_owned())).await.is_ok() {

        } else {
            println!("Could not send ping {}!", who);
            // no Error here since the only thing we can do is to close the connection.
            // If we can not send messages, there is no way to salvage the statemachine anyway.
            return;
        }

        loop {
            for item in &playlist {
                let sleep;
                // println!("{item:?}");
                let msg = match item {
                    store::store::PlaylistItem::Website { name, settings } => {
                        println!("Sending {name:?} website to {:?}", hello.hostname);
                        sleep = settings.duration;
                        DisplayPayload::Website { data: WebsitePayload { content: settings.url.clone() } }
                    },
                };
                sender.lock().await.send(Message::Text(serde_json::to_string(&Payload::Display(msg)).unwrap())).await.unwrap();
                tokio::time::sleep(Duration::from_secs(sleep)).await;
            }
        };
    });

    tokio::select! {
        _ = (&mut hb) => send.abort(),
        _ = (&mut send) => hb.abort(),
    };

    println!("Done!");
}

fn heart_beat(sender: Arc<Mutex<SplitSink<WebSocket, Message>>>) -> tokio::task::JoinHandle<()> {
    //TODO: must check timeout on receiving a response
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(time::Duration::from_secs(2));
        loop {
            interval.tick().await;
            let mut socket = sender.lock().await;
            match socket.send(Message::Ping(vec![])).await {
                Ok(_) => println!("Sent Ping"),
                Err(_) => {
                    println!("Could not ping, fuck");
                    match socket.close().await {
                        Ok(_) => println!("hej"),
                        Err(_) => println!("nej"),
                    }
                    return;
                },
            };
        };
    })
}
