use std::{net::SocketAddr, sync::Arc};

use axum::{Router, routing::get, extract::{WebSocketUpgrade, ConnectInfo, State, Path}, response::IntoResponse, Json, ServiceExt};
use hyper::StatusCode;
use store::store::Store;
use tokio::sync::oneshot;
use tower_http::normalize_path::NormalizePath;
use uuid::Uuid;

use crate::connection::connection::client_connection;

mod store;
mod connection;

impl From<(u8, String)> for response::Error {
    fn from(value: (u8, String)) -> Self {
        response::Error { code: value.0, message: value.1 }
    }
}

#[tokio::main]
async fn main() {
    let store = Arc::new(Store::new().await);

    let store_copy = store.clone();
    let (tx, rx) = oneshot::channel::<()>();

    tokio::spawn(async move {
        store_copy.schedule_loop(tx).await;
    });
    
    rx.await.unwrap();
    
    println!("{}", store.to_string().await);

    let app = NormalizePath::trim_trailing_slash(
        Router::new()
            .nest("/api", Router::new()
                .route("/display", get(get_displays))
                .route("/playlist", get(get_playlists))
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

mod response {
    use axum::Json;
    use hyper::StatusCode;
    use serde::Serialize;
    use uuid::Uuid;

    use crate::store::{store, schedule};

    pub type Response = Result<Json<Payload>, (StatusCode, Json<Error>)>;

    #[derive(Serialize)]
    #[serde(tag = "type", content = "content")]
    pub enum Payload {
        Display(Vec<Display>),
        Playlist(Vec<Playlist>),
        Schedule(Vec<Schedule>),
    }

    #[derive(Serialize)]
    pub struct Display {
        pub uuid: Uuid,
        pub name: String,
        pub schedule: Uuid
    }

    impl From<(Uuid, store::Display)> for Display {
        fn from((uuid, d): (Uuid, store::Display)) -> Self {
            Self { uuid, name: d.name, schedule: d.schedule }
        }
    }

    #[derive(Serialize)]
    pub struct Playlist {
        pub uuid: Uuid,
        pub name: String,
        pub items: Vec<store::PlaylistItem>,
    }

    impl From<(Uuid, store::Playlist)> for Playlist {
        fn from((uuid, p): (Uuid, store::Playlist)) -> Self {
            Self { uuid, name: p.name, items: p.items }
        }
    }

    #[derive(Serialize)]
    pub struct Schedule {
        pub uuid: Uuid,
        pub name: String,
        pub playlist: Uuid,
        pub scheduled: Option<Vec<schedule::ScheduledPlaylistInput>>
    }

    impl From<(Uuid, schedule::Schedule)> for Schedule {
        fn from((uuid, s): (Uuid, schedule::Schedule)) -> Self {
            let s = schedule::ScheduleInput::from(s);
            Self { uuid, name: s.name, playlist: s.playlist, scheduled: s.scheduled }
        }
    }
    
    #[derive(Serialize)]
    pub struct Error {
        pub code: u8,
        pub message: String
    }
}

async fn get_displays(State(store): State<Arc<Store>>) -> response::Response {
    Ok(Json(response::Payload::Display(
            store
                .read()
                .await
                .displays
                .iter()
                .map(|(u, d)| (*u, d.clone()).into())
                .collect()
    )))
}

async fn get_playlists(State(store): State<Arc<Store>>) -> response::Response {
    Ok(Json(response::Payload::Playlist(
        store
            .read()
            .await
            .playlists
            .iter()
            .map(|(u, p)| (*u, p.clone()).into())
            .collect::<Vec<_>>()
    )))
}

async fn get_schedules(State(store): State<Arc<Store>>) -> response::Response {
    Ok(Json(response::Payload::Schedule(
        store
            .read()
            .await
            .schedules
            .iter()
            .map(|(u, s)| (*u, s.clone()).into())
            .collect::<Vec<_>>()
    )))
}

async fn set_display_schedule(State(store): State<Arc<Store>>, Path((display, schedule)): Path<(Uuid, Uuid)>) -> Result<String, (StatusCode, Json<response::Error>)> {
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
    ws.on_upgrade(move |socket| client_connection(socket, addr, store))
}