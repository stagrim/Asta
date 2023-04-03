use core::time;
use std::{net::SocketAddr, sync::Arc, collections::HashMap, time::Duration};

use axum::{Router, routing::get, extract::{WebSocketUpgrade, ConnectInfo, ws::{Message, WebSocket}, State}, response::IntoResponse};
use futures_util::{StreamExt, SinkExt, stream::SplitSink};
use store::store::{Store, Content};
use tokio::{sync::Mutex};

mod store;

struct ServerState {
    content: Mutex<Content>,
}

#[tokio::main]
async fn main() {
    let store = Store::new();

    
    let loaded = store.load().await;
    println!("{:#?}", loaded);

    let server_state = Arc::new(ServerState { content: Mutex::new(loaded) });

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
    State(state): State<Arc<ServerState>>,
) -> impl IntoResponse {
    println!("{addr} connected.");
    // finalize the upgrade process by returning upgrade callback.
    // we can customize the callback by sending additional info such as address.
    ws.on_upgrade(move |socket| handle_socket(socket, addr, state))
}

async fn handle_socket(socket: WebSocket, who: SocketAddr, state: Arc<ServerState>) {
    let (sender, mut reciever) = socket.split();
    let sender = Arc::new(Mutex::new(sender));
    let mut hb = heart_beat(sender.clone());
    //send a ping (unsupported by some browsers) just to kick things off and get a response

    let mut send = tokio::spawn(async move {
        let content = state.content.lock().await;
        let schedule = &content.displays.get("631f6175-6829-4e16-ad7f-cee6105f4c39").unwrap().schedule;
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
                println!("{item:?}");
                match item {
                    store::store::PlaylistItem::Website { name: _, settings } => {
                        sleep = settings.duration;
                    },
                };
                tokio::time::sleep(Duration::from_secs(sleep)).await;
            }
        }
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
