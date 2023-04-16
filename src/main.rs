use std::{net::SocketAddr, sync::Arc};

use axum::{Router, routing::get, extract::{WebSocketUpgrade, ConnectInfo, State, Path}, response::IntoResponse, Json, ServiceExt};
use hyper::StatusCode;
use serde::Serialize;
use store::store::{Store, Content};
use tokio::sync::{Mutex, watch::{self, Receiver, Sender}};
use tower_http::normalize_path::NormalizePath;

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

#[derive(Serialize)]
struct ErrorResponse {
    code: u8,
    message: String
}

impl From<(u8, String)> for ErrorResponse {
    fn from(value: (u8, String)) -> Self {
        ErrorResponse { code: value.0, message: value.1 }
    }
}

#[tokio::main]
async fn main() {
    let store = Store::new();
    
    let loaded = store.load().await;
    println!("{:#?}", loaded);

    let (tx, rx) = watch::channel(loaded);

    // Must wrap tx in Mutex since it does not implement the Copy trait like rx. 
    // Watch still makes since for this project over broadcast , since no history is needed
    let tx = Arc::new(Mutex::new(tx));

    let server_state = ServerState { rx, tx };

    let app = NormalizePath::trim_trailing_slash(
        Router::new()
            .nest("/api", Router::new()
                .route("/schedule/:display/:schedule", get(set_display_schedule))
                .route("/schedule", get(get_schedules))
            )
            .route("/", get(ws_handler))
            .with_state(server_state)
    );

    let addr = SocketAddr::from(([0, 0, 0, 0], 8040));
    println!("listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service_with_connect_info::<SocketAddr>())
        .await.unwrap();
}

#[derive(Serialize)]
struct Schedule {
    uuid: UUID,
    name: String,
    playlist: UUID
}

async fn get_schedules(State(state): State<ServerState>) -> Json<Vec<Schedule>> {
    Json(
        state.rx
        .borrow().schedules.clone()
        .into_iter()
        .map(|(uuid, s)| Schedule {
            uuid,
            name: s.name,
            playlist: s.playlist
        }).collect()
    )
}

async fn set_display_schedule(State(state): State<ServerState>, Path((display, schedule)): Path<(UUID, UUID)>) -> Result<String, (StatusCode, Json<ErrorResponse>)> {
    let rx = state.rx;
    let schedule_exists = rx.borrow().schedules.contains_key(&schedule);
    let display_exists = rx.borrow().displays.contains_key(&display);

    if schedule_exists && display_exists && rx.borrow().displays.get(&display).unwrap().schedule != schedule {
        let mut content = rx.borrow().clone();
        let res = format!("set display {display} to schedule {schedule}");
        content.displays.entry(display).and_modify(|d| d.schedule = schedule);
        state.tx.lock().await.send(content).unwrap();
        return Ok(res)
    }

    Err((StatusCode::BAD_REQUEST, Json({
        if !schedule_exists {
            (1, format!("{schedule} is not a defined schedule"))
        } else if !display_exists {
            (2, format!("{display} is not a defined display"))
        } else {
            (3, format!("Display is already set to specified schedule"))
        }}.into()
    )))
}

async fn ws_handler(ws: WebSocketUpgrade, ConnectInfo(addr): ConnectInfo<SocketAddr>, State(state): State<ServerState>) -> impl IntoResponse {
    println!("{addr} connected.");
    ws.on_upgrade(move |socket| client_connection(socket, addr, state.rx.clone()))
}