use std::{net::SocketAddr, sync::Arc};

use axum::{Router, routing::get, extract::{WebSocketUpgrade, ConnectInfo, State, Path}, response::IntoResponse};
use hyper::StatusCode;
use store::store::{Store, Content};
use tokio::{sync::{Mutex, watch::{self, Receiver, Sender}}};

use crate::connection::connection::client_connection;

mod store;
mod connection;

type UUID = String;

#[derive(Clone)]
struct ServerState {
    // content: Mutex<Content>,
    rx: Receiver<Content>,
    tx: Arc<Mutex<Sender<Content>>>
}

#[tokio::main]
async fn main() {
    let store = Store::new();

    
    let loaded = store.load().await;
    println!("{:#?}", loaded);

    let (tx, rx) = watch::channel(loaded);

    let tx = Arc::new(Mutex::new(tx));

    let server_state = ServerState { rx, tx };

    let app = Router::new()
        .route("/", get(ws_handler))
        .route("/api/set/schedule/:display/:schedule", get(set_display_schedule))
        .with_state(server_state);

    // run it with hyper
    let addr = SocketAddr::from(([0, 0, 0, 0], 8040));
    println!("listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service_with_connect_info::<SocketAddr>())
        .await
        .unwrap();
}

async fn set_display_schedule(State(state): State<ServerState>, Path((display, schedule)): Path<(UUID, UUID)>) -> StatusCode {
    let rx = state.rx.clone();
    if rx.borrow().schedules.contains_key(&schedule) && rx.borrow().displays.contains_key(&display) {
        let mut content = rx.borrow().clone();
        content.displays.entry(display).and_modify(|d| d.schedule = schedule);
        state.tx.lock().await.send(content).unwrap();
        StatusCode::OK
    } else {
        StatusCode::BAD_REQUEST
    }
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(state): State<ServerState>,
) -> impl IntoResponse {
    println!("{addr} connected.");
    ws.on_upgrade(move |socket| client_connection(socket, addr, state.rx.clone()))
}