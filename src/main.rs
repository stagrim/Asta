use std::{net::SocketAddr, sync::Arc};

use axum::{Router, routing::get, extract::{WebSocketUpgrade, ConnectInfo, State, Path}, response::IntoResponse, Json, ServiceExt};
use hyper::StatusCode;
use serde::Serialize;
use store::store::Store;
use tokio_util::sync::CancellationToken;
use tower_http::normalize_path::NormalizePath;
use uuid::Uuid;

use crate::connection::connection::client_connection;

mod store;
mod connection;

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
    let store = Arc::new(Store::new().await);

    let store_copy = store.clone();
    let updated_active_playlists = CancellationToken::new();
    let done = updated_active_playlists.clone();

    tokio::spawn(async move {
        store_copy.schedules(done).await;
    });
    updated_active_playlists.cancelled().await;
    
    println!("{}", store.to_string().await);

    let app = NormalizePath::trim_trailing_slash(
        Router::new()
            .nest("/api", Router::new()
                .route("/schedule/:display/:schedule", get(set_display_schedule))
                .route("/schedule", get(get_schedules))
            )
            .route("/", get(ws_handler))
            .with_state(store)
    );

    let addr = SocketAddr::from(([0, 0, 0, 0], 8040));
    println!("listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service_with_connect_info::<SocketAddr>())
        .await.unwrap();
}

#[derive(Serialize)]
struct ScheduleResponse {
    uuid: Uuid,
    name: String,
    playlist: Uuid
}

async fn get_schedules(State(store): State<Arc<Store>>) -> Json<Vec<ScheduleResponse>> {
    Json(
        store.read().await
            .schedules
            .iter()
            .map(|(uuid, s)| ScheduleResponse {
                uuid: uuid.clone(),
                name: s.name.clone(),
                playlist: s.playlist
            }).collect()
    )
}

async fn set_display_schedule(State(store): State<Arc<Store>>, Path((display, schedule)): Path<(Uuid, Uuid)>) -> Result<String, (StatusCode, Json<ErrorResponse>)> {
    let read = store.read().await;
    let schedule_exists = read.schedules.contains_key(&schedule);
    let display_exists = read.displays.contains_key(&display);
    let set_to_schedule = read.displays.get(&display).unwrap().schedule != schedule;
    drop(read);

    if schedule_exists && display_exists && set_to_schedule {
        let res = format!("set display {display} to schedule {schedule}");
        store.update_display_schedule(display, schedule).await;
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

async fn ws_handler(ws: WebSocketUpgrade, ConnectInfo(addr): ConnectInfo<SocketAddr>, State(store): State<Arc<Store>>) -> impl IntoResponse {
    println!("{addr} connected.");
    ws.on_upgrade(move |socket| client_connection(socket, addr, store))
}