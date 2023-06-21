use std::{net::SocketAddr, sync::Arc, vec};

use axum::{Router, routing::{get, post, put, delete}, extract::{WebSocketUpgrade, ConnectInfo, State, Path}, response::{IntoResponse}, Json, ServiceExt};
use axum_macros::debug_handler;
use hyper::StatusCode;
use store::store::Store;
use tokio::sync::oneshot;
use tower_http::normalize_path::NormalizePath;
use uuid::Uuid;

use crate::connection::connection::client_connection;

mod store;
mod connection;

impl From<(u8, String)> for read::Error {
    fn from(value: (u8, String)) -> Self {
        read::Error { code: value.0, message: value.1 }
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
                .route("/display", get(read_displays))
                .route("/display", post(create_display))
                .route("/display/:uuid", put(update_display))
                .route("/display/:uuid", delete(delete_display))
                .route("/playlist", get(read_playlists))
                .route("/schedule", get(read_schedules))
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

mod read {
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
    #[serde(tag = "type")]
    pub struct Error {
        pub code: u8,
        pub message: String
    }
}

mod create {
    use serde::Deserialize;
    use uuid::Uuid;

    pub use crate::read::{Schedule, Playlist, Error, Response};

    #[derive(Deserialize)]
    pub struct Display {
        pub name: String,
        pub schedule: Uuid
    }
}

mod update {
    pub use crate::read::{Response, Payload};
    pub use crate::create::{Display};
}

#[debug_handler]
async fn create_display(State(store): State<Arc<Store>>, Json(display): Json<create::Display>) -> read::Response {
    println!("[Api] Creating Display with name {} and schedule {}", display.name, display.schedule);
    let read = store.read().await;
    if let Some((uuid, _)) = read.displays.iter().find(|(_, d)| d.name == display.name) {
        println!("[Api] Name is already used by Display {}", uuid);
        return Err((StatusCode::BAD_REQUEST,
            Json((1, format!("Avoid using the name {} as it is already used by another display", display.name)).into())
        ))
    }

    drop(read);
    let uuid = Uuid::new_v4();
    println!("[Api] Generated Uuid {uuid} for new Display");
    store.create_display(uuid, display.name, display.schedule).await;

    if let Some(d) = store.read().await.displays.get(&uuid) {
        println!("[Api] Created Display {uuid}");
        Ok(Json(read::Payload::Display(vec![(uuid, d.clone()).into()])))
    } else {
        println!("[Api] No Display with {} could be found while reading after write", uuid);
        Err((StatusCode::INTERNAL_SERVER_ERROR,
            Json((2, format!("Something went wrong with the creation")).into())
        ))
    }
}

async fn read_displays(State(store): State<Arc<Store>>) -> read::Response {
    Ok(Json(read::Payload::Display(
            store
                .read()
                .await
                .displays
                .iter()
                .map(|(u, d)| (*u, d.clone()).into())
                .collect()
    )))
}

async fn read_playlists(State(store): State<Arc<Store>>) -> read::Response {
    Ok(Json(read::Payload::Playlist(
        store
            .read()
            .await
            .playlists
            .iter()
            .map(|(u, p)| (*u, p.clone()).into())
            .collect::<Vec<_>>()
    )))
}

async fn read_schedules(State(store): State<Arc<Store>>) -> read::Response {
    Ok(Json(read::Payload::Schedule(
        store
            .read()
            .await
            .schedules
            .iter()
            .map(|(u, s)| (*u, s.clone()).into())
            .collect::<Vec<_>>()
    )))
}

#[debug_handler]
async fn update_display(State(store): State<Arc<Store>>, Path(uuid): Path<Uuid>, Json(display): Json<update::Display>) -> update::Response {
    println!("[Api] Updating Display {uuid}");
    let read = store.read().await;
    if !read.displays.contains_key(&uuid) {
        println!("[Api] No display with {uuid} was found");
        return Err((StatusCode::BAD_REQUEST, Json((1, format!("No Display with the Uuid {uuid} was found")).into())))
    }
    if let Some((uuid, _)) = read.displays.iter().find(|(&u, d)| d.name == display.name && u != uuid) {
        println!("[Api] Name is already used by Display {}", uuid);
        return Err((StatusCode::BAD_REQUEST, Json((2, format!("Avoid using the name {} as it is already used by another display", display.name)).into())))
    }
    drop (read);

    store.update_display(uuid, display.name, display.schedule).await;
    
    if let Some(d) = store.read().await.displays.get(&uuid) {
        println!("[Api] Updated and read Display {uuid}");
        Ok(Json(update::Payload::Display(vec![(uuid, d.clone()).into()])))
    } else {
        println!("[Api] Could not find Display with {uuid} after update");
        Err((StatusCode::INTERNAL_SERVER_ERROR, Json((3, format!("Could not find Display with {uuid} after update")).into())))
    }
}

#[debug_handler]
async fn delete_display(State(store): State<Arc<Store>>, Path(uuid): Path<Uuid>) -> read::Response {
    println!("[Api] Deleting Display {uuid}");
    let res: read::Response;
    if let Some(d) = store.read().await.displays.get(&uuid) {
        res = Ok(Json(read::Payload::Display(vec![(uuid, d.clone()).into()])));
    } else {
        println!("[Api] No display with {uuid} was found");
        return Err((StatusCode::BAD_REQUEST,
            Json((1, format!("No Display with the Uuid {uuid} was found")).into())
        ))
    }

    store.delete_display(uuid).await;
    println!("[Api] Deleted Display {uuid}");
    res
}

//TODO: Add Delete display here

// async fn set_display_schedule(State(store): State<Arc<Store>>, Path((display, schedule)): Path<(Uuid, Uuid)>) -> Result<String, (StatusCode, Json<response::Error>)> {
//     let read = store.read().await;
//     let schedule_exists = read.schedules.contains_key(&schedule);
//     let display_exists = read.displays.contains_key(&display);
//     let set_to_schedule = read.displays.get(&display).unwrap().schedule != schedule;
//     drop(read);

//     if schedule_exists && display_exists && set_to_schedule {
//         let res = format!("set display {display} to schedule {schedule}");
//         store.update_display_schedule(display, schedule).await;
//         return Ok(res)
//     }

//     Err((StatusCode::BAD_REQUEST, Json({
//         if !schedule_exists {
//             (1, format!("{schedule} is not a defined schedule"))
//         } else if !display_exists {
//             (2, format!("{display} is not a defined display"))
//         } else {
//             (3, format!("Display is already set to specified schedule"))
//         }}.into()
//     )))
// }

async fn ws_handler(ws: WebSocketUpgrade, ConnectInfo(addr): ConnectInfo<SocketAddr>, State(store): State<Arc<Store>>) -> impl IntoResponse {
    ws.on_upgrade(move |socket| client_connection(socket, addr, store))
}
