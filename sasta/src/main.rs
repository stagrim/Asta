use std::{net::SocketAddr, sync::Arc, vec, collections::HashSet};

use axum::{Router, routing::{get, post, put, delete}, extract::{WebSocketUpgrade, ConnectInfo, State, Path}, response::IntoResponse, Json, ServiceExt};
use axum_macros::debug_handler;
use hyper::StatusCode;
use store::store::Store;
use tokio::sync::{oneshot, Mutex};
use tower_http::normalize_path::NormalizePath;
use tracing::{Level, error, info, info_span};
use tracing_subscriber::{FmtSubscriber, fmt::format::FmtSpan};
use uuid::Uuid;

use crate::{connection::connection::client_connection, file_server::file_server::{get_files, FileServer, add_files}};

mod store;
mod connection;
mod file_server;

impl From<(u8, String)> for read::Payload {
    fn from(value: (u8, String)) -> Self {
        read::Payload::Error { code: value.0, message: value.1 }
    }
}

// TODO: Race conditions possible in like all API routes. Use Mutex instead?

#[derive(Clone)]
pub struct AppState {
    store: Arc<Store>,
    file_server: Arc<Mutex<FileServer>>,
}

#[tokio::main]
async fn main() {
    let subscriber = FmtSubscriber::builder()
        .with_span_events(FmtSpan::NEW)
        .with_max_level(Level::TRACE)
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    let store = Arc::new(Store::new().await);
    let file_server = Arc::new(Mutex::new(FileServer::new().await));

    let store_copy = store.clone();
    let (tx, rx) = oneshot::channel::<()>();

    tokio::spawn(async move {
        store_copy.schedule_loop(tx).await;
    });

    rx.await.unwrap();

    info!("{}", store.to_string().await);

    let app_state = AppState { store, file_server };

    let app = NormalizePath::trim_trailing_slash(
        Router::new()
            .nest("/api", Router::new()
                .nest("/display", Router::new()
                    .route("/", get(read_display))
                    .route("/", post(create_display))
                    .route("/:uuid", put(update_display))
                    .route("/:uuid", delete(delete_display))
                )
                .nest("/playlist", Router::new()
                    .route("/", post(create_playlist))
                    .route("/", get(read_playlist))
                    .route("/:uuid", put(update_playlist))
                    .route("/:uuid", delete(delete_playlist))
                )
                .nest("/schedule", Router::new()
                    .route("/", post(create_schedule))
                    .route("/", get(read_schedule))
                    .route("/:uuid", put(update_schedule))
                    .route("/:uuid", delete(delete_schedule))
                )
                .nest("/files", Router::new()
                    .route("/", get(get_files))
                    .route("/", post(add_files))
                )
            )
            .nest("/files", Router::new()
            )
            .route("/", get(ws_handler))
            .with_state(app_state)
    );

    let addr = SocketAddr::from(([0, 0, 0, 0], 8040));
    info!("listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service_with_connect_info::<SocketAddr>())
        .await.unwrap();
}

mod read {
    use axum::Json;
    use hyper::StatusCode;
    use serde::Serialize;
    use ts_rs::TS;
    use uuid::Uuid;

    use crate::store::{store, schedule};

    pub type Response = Result<Json<Payload>, (StatusCode, Json<Payload>)>;

    #[derive(Serialize, TS)]
    #[serde(tag = "type", content = "content")]
    #[ts(export, export_to = "api_bindings/read/")]
    pub enum Payload {
        Display(Vec<Display>),
        Playlist(Vec<Playlist>),
        Schedule(Vec<Schedule>),
        Error {
            code: u8,
            message: String
        }
    }

    #[derive(Serialize, TS)]
    #[ts(export, export_to = "api_bindings/read/")]
    pub struct Display {
        #[ts(type = "string")]
        pub uuid: Uuid,
        pub name: String,
        #[ts(type = "string")]
        pub schedule: Uuid
    }

    impl From<(Uuid, store::Display)> for Display {
        fn from((uuid, d): (Uuid, store::Display)) -> Self {
            Self { uuid, name: d.name, schedule: d.schedule }
        }
    }

    #[derive(Serialize, TS)]
    #[ts(export, export_to = "api_bindings/read/")]
    pub struct Playlist {
        #[ts(type = "string")]
        pub uuid: Uuid,
        pub name: String,
        pub items: Vec<store::PlaylistItem>,
    }

    impl From<(Uuid, store::Playlist)> for Playlist {
        fn from((uuid, p): (Uuid, store::Playlist)) -> Self {
            Self { uuid, name: p.name, items: p.items }
        }
    }

    #[derive(Serialize, TS)]
    #[ts(export, export_to = "api_bindings/read/")]
    pub struct Schedule {
        #[ts(type = "string")]
        pub uuid: Uuid,
        pub name: String,
        #[ts(type = "string")]
        pub playlist: Uuid,
        pub scheduled: Option<Vec<schedule::ScheduledPlaylistInput>>
    }

    impl From<(Uuid, schedule::Schedule)> for Schedule {
        fn from((uuid, s): (Uuid, schedule::Schedule)) -> Self {
            let s = schedule::ScheduleInput::from(s);
            Self { uuid, name: s.name, playlist: s.playlist, scheduled: s.scheduled }
        }
    }
}

mod create {
    use serde::Deserialize;
    use ts_rs::TS;
    use uuid::Uuid;

    pub use crate::read::Response;

    #[derive(Debug, Deserialize, TS)]
    #[ts(export, export_to = "api_bindings/create/", rename = "CreateDisplay")]
    pub struct Display {
        #[ts(type = "string", optional)]
        pub uuid: Option<Uuid>,
        pub name: String,
        #[ts(type = "string")]
        pub schedule: Uuid
    }

    #[derive(Deserialize, TS)]
    #[ts(export, export_to = "api_bindings/create/", rename = "CreatePlaylist")]
    pub struct Playlist {
        pub name: String
    }

    #[derive(Deserialize, TS)]
    #[ts(export, export_to = "api_bindings/create/", rename = "CreateSchedule")]
    pub struct Schedule {
        pub name: String,
        #[ts(type = "string")]
        pub playlist: Uuid,
    }
}

mod update {
    use serde::Deserialize;
    use ts_rs::TS;
    use uuid::Uuid;

    pub use crate::read::{Response, Payload};
    use crate::store::{store, schedule};

    #[derive(Deserialize, TS)]
    #[ts(export, export_to = "api_bindings/update/", rename = "UpdateDisplay")]
    pub struct Display {
        pub name: String,
        #[ts(type = "string")]
        pub schedule: Uuid
    }

    #[derive(Deserialize, TS)]
    #[ts(export, export_to = "api_bindings/update/", rename = "UpdatePlaylist")]
    pub struct Playlist {
        pub name: String,
        pub items: Vec<store::PlaylistItem>,
    }

    #[derive(Deserialize, TS)]
    #[ts(export, export_to = "api_bindings/update/", rename = "UpdateSchedule")]
    pub struct Schedule {
        pub name: String,
        #[ts(type = "string")]
        pub playlist: Uuid,
        #[ts(optional)]
        pub scheduled: Option<Vec<schedule::ScheduledPlaylistInput>>
    }
}

#[debug_handler]
async fn create_display(State(state): State<AppState>, Json(disp): Json<create::Display>) -> read::Response {
    info_span!("[Api] Creating Display", display = ?disp);
    let store = state.store;
    let read = store.read().await;
    if let Some((uuid, _)) = read.displays.iter().find(|(_, d)| d.name == disp.name) {
        error!("[Api] Name is already used by Display {}", uuid);
        return Err((StatusCode::BAD_REQUEST,
            Json((1, format!("Avoid using the name {} as it is already used by another display", disp.name)).into())
        ))
    }

    if let Some(uuid) = disp.uuid {
        if read.displays.contains_key(&uuid) {
            error!("[Api] Uuid is already used by another Display");
            return Err((StatusCode::BAD_REQUEST,
                Json((2, format!("Avoid using the Uuid {} as it is already used by another display", disp.name)).into())
            ))
        }
    }
    drop(read);

    let uuid = match disp.uuid {
        Some(u) => u,
        None => Uuid::new_v4()
    };
    info!("[Api] Using Uuid {uuid} for new Display");

    store.create_display(uuid, disp.name, disp.schedule).await;

    return if let Some(d) = store.read().await.displays.get(&uuid) {
        info!("[Api] Created Display {uuid}");
        Ok(Json(read::Payload::Display(vec![(uuid, d.clone()).into()])))
    } else {
        error!("[Api] No Display with {} could be found while reading after write", uuid);
        Err((StatusCode::INTERNAL_SERVER_ERROR,
            Json((3, format!("Something went wrong with the creation")).into())
        ))
    };
}

async fn create_playlist(State(state): State<AppState>, Json(playlist): Json<create::Playlist>) -> create::Response {
    info!("[Api] Creating Playlist with name {}", playlist.name);
    let store = state.store;
    let read = store.read().await;
    if let Some((uuid, _)) = read.playlists.iter().find(|(_, p)| p.name == playlist.name) {
        error!("[Api] Name is already used by Playlist {}", uuid);
        return Err((StatusCode::BAD_REQUEST,
            Json((1, format!("Avoid using the name {} as it is already used by another Playlist", playlist.name)).into())
        ))
    }

    drop(read);
    let uuid = Uuid::new_v4();
    info!("[Api] Generated Uuid {uuid} for new Playlist");
    store.create_playlist(uuid, playlist.name).await;

    return if let Some(p) = store.read().await.playlists.get(&uuid) {
        info!("[Api] Created Playlist {uuid}");
        Ok(Json(read::Payload::Playlist(vec![(uuid, p.clone()).into()])))
    } else {
        error!("[Api] No Playlist with {uuid} could be found while reading after write");
        Err((StatusCode::INTERNAL_SERVER_ERROR,
            Json((2, format!("Something went wrong with the creation")).into())
        ))
    };
}

async fn create_schedule(State(state): State<AppState>, Json(schedule): Json<create::Schedule>) -> create::Response {
    info!("[Api] Creating Schedule with name {}", schedule.name);
    let store = state.store;
    let read = store.read().await;
    if let Some((uuid, _)) = read.schedules.iter().find(|(_, s)| s.name == schedule.name) {
        error!("[Api] Name is already used by Schedule {}", uuid);
        return Err((StatusCode::BAD_REQUEST,
            Json((1, format!("Avoid using the name {} as it is already used by another Schedule", schedule.name)).into())
        ))
    }

    drop(read);
    let uuid = Uuid::new_v4();
    info!("[Api] Generated Uuid {uuid} for new Schedule");
    store.create_schedule(uuid, schedule.name, schedule.playlist).await;

    return if let Some(s) = store.read().await.schedules.get(&uuid) {
        info!("[Api] Created Schedule {uuid}");
        Ok(Json(read::Payload::Schedule(vec![(uuid, s.clone()).into()])))
    } else {
        error!("[Api] No Schedule with {uuid} could be found while reading after write");
        Err((StatusCode::INTERNAL_SERVER_ERROR,
            Json((2, format!("Something went wrong with the creation")).into())
        ))
    };
}

async fn read_display(State(state): State<AppState>) -> read::Response {
    return Ok(Json(read::Payload::Display(
            state.store
                .read()
                .await
                .displays
                .iter()
                .map(|(u, d)| (*u, d.clone()).into())
                .collect()
    )));
}

async fn read_playlist(State(state): State<AppState>) -> read::Response {
    return Ok(Json(read::Payload::Playlist(
        state.store
            .read()
            .await
            .playlists
            .iter()
            .map(|(u, p)| (*u, p.clone()).into())
            .collect::<Vec<_>>()
    )));
}

async fn read_schedule(State(state): State<AppState>) -> read::Response {
    return Ok(Json(read::Payload::Schedule(
        state.store
            .read()
            .await
            .schedules
            .iter()
            .map(|(u, s)| (*u, s.clone()).into())
            .collect::<Vec<_>>()
    )));
}

async fn update_display(State(state): State<AppState>, Path(uuid): Path<Uuid>, Json(display): Json<update::Display>) -> update::Response {
    info!("[Api] Updating Display {uuid}");
    let store = state.store;
    let read = store.read().await;
    if !read.displays.contains_key(&uuid) {
        error!("[Api] No display with {uuid} was found");
        return Err((StatusCode::BAD_REQUEST, Json((1, format!("No Display with the Uuid {uuid} was found")).into())))
    }
    if let Some((uuid, _)) = read.displays.iter().find(|(&u, d)| d.name == display.name && u != uuid) {
        error!("[Api] Name is already used by Display {}", uuid);
        return Err((StatusCode::BAD_REQUEST, Json((2, format!("Avoid using the name {} as it is already used by another display", display.name)).into())))
    }
    drop(read);

    store.update_display(uuid, display.name, display.schedule).await;

    return if let Some(d) = store.read().await.displays.get(&uuid) {
        info!("[Api] Updated and read Display {uuid}");
        Ok(Json(update::Payload::Display(vec![(uuid, d.clone()).into()])))
    } else {
        error!("[Api] Could not find Display with {uuid} after update");
        Err((StatusCode::INTERNAL_SERVER_ERROR, Json((3, format!("Could not find Display with {uuid} after update")).into())))
    };
}

async fn update_playlist(State(state): State<AppState>, Path(uuid): Path<Uuid>, Json(playlist): Json<update::Playlist>) -> update::Response {
    info!("[Api] Updating Playlist {uuid}");
    let store = state.store;
    let read = store.read().await;
    if !read.playlists.contains_key(&uuid) {
        error!("[Api] No Playlist with {uuid} was found");
        return Err((StatusCode::BAD_REQUEST, Json((1, format!("No Playlist with the Uuid {uuid} was found")).into())))
    }
    if let Some((uuid, _)) = read.playlists.iter().find(|(&u, p)| p.name == playlist.name && u != uuid) {
        error!("[Api] Name is already used by Playlist {}", uuid);
        return Err((StatusCode::BAD_REQUEST, Json((2, format!("Avoid using the name {} as it is already used by another Playlist", playlist.name)).into())))
    }
    drop(read);

    store.update_playlist(uuid, playlist.name, playlist.items).await;

    return if let Some(p) = store.read().await.playlists.get(&uuid) {
        info!("[Api] Updated and read Playlist {uuid}");
        Ok(Json(update::Payload::Playlist(vec![(uuid, p.clone()).into()])))
    } else {
        error!("[Api] Could not find Playlist with {uuid} after update");
        Err((StatusCode::INTERNAL_SERVER_ERROR, Json((3, format!("Could not find Playlist with {uuid} after update")).into())))
    };
}

async fn update_schedule(State(state): State<AppState>, Path(uuid): Path<Uuid>, Json(schedule): Json<update::Schedule>) -> update::Response {
    info!("[Api] Updating Schedule {uuid}");
    let store = state.store;
    let read = store.read().await;
    if !read.schedules.contains_key(&uuid) {
        error!("[Api] No Schedule with {uuid} was found");
        return Err((StatusCode::BAD_REQUEST, Json((1, format!("No Schedule with the Uuid {uuid} was found")).into())))
    }
    if let Some((uuid, _)) = read.schedules.iter().find(|(&u, s)| s.name == schedule.name && u != uuid) {
        error!("[Api] Name is already used by Schedule {}", uuid);
        return Err((StatusCode::BAD_REQUEST, Json((2, format!("Avoid using the name {} as it is already used by another Schedule", schedule.name)).into())))
    }

    if let Some(scheduled) = &schedule.scheduled {
        let mut uniq = HashSet::new();
        uniq.insert(schedule.playlist);
        // Checks if any playlist Uuid is a duplicate
        if !scheduled.iter().all(|s| uniq.insert(s.playlist)) {
            error!("[Api] Schedule contains duplicate Playlists");
            return Err((StatusCode::BAD_REQUEST, Json((3, format!("Must not use the same Playlist more than once in a Schedule to avoid server meltdown")).into())))
        }
    }
    drop(read);

    if let Err(e) = store.update_schedule(uuid, schedule.name, schedule.playlist, schedule.scheduled.unwrap_or(vec![])).await {
        error!("[Api] Schedule update failed with error: {e}");
        return Err((StatusCode::INTERNAL_SERVER_ERROR, Json((4, format!("{e}")).into())))
    }

    return if let Some(s) = store.read().await.schedules.get(&uuid) {
        info!("[Api] Updated and read Schedule {uuid}");
        Ok(Json(update::Payload::Schedule(vec![(uuid, s.clone()).into()])))
    } else {
        error!("[Api] Could not find Schedule with {uuid} after update");
        Err((StatusCode::INTERNAL_SERVER_ERROR, Json((5, format!("Could not find Schedule with {uuid} after update to avoid server meltdown")).into())))
    };
}

async fn delete_display(State(state): State<AppState>, Path(uuid): Path<Uuid>) -> read::Response {
    info!("[Api] Deleting Display {uuid}");
    let store = state.store;
    let res;
    if let Some(d) = store.read().await.displays.get(&uuid) {
        res = Ok(Json(read::Payload::Display(vec![(uuid, d.clone()).into()])));
    } else {
        error!("[Api] No display with {uuid} was found");
        return Err((StatusCode::BAD_REQUEST,
            Json((1, format!("No Display with the Uuid {uuid} was found")).into())
        ))
    }

    store.delete_display(uuid).await;
    info!("[Api] Deleted Display {uuid}");
    res
}

async fn delete_playlist(State(state): State<AppState>, Path(uuid): Path<Uuid>) -> read::Response {
    info!("[Api] Deleting Playlist {uuid}");
    let res;
    let store = state.store;
    let read = store.read().await;

    let dependant_schedules = read.schedules.iter()
        .filter_map(|(_, s)|
            if s.all_playlists().iter().any(|&p| p == &uuid) {
                Some(s.name.clone())
            } else {
                None
            }
        ).collect::<Vec<_>>();
    if dependant_schedules.len() > 0 {
        return Err((StatusCode::BAD_REQUEST,
            Json((1, format!("Unable to delete playlist since the Schedules ({}) depend on it",
                dependant_schedules.join(", ")
            )).into())
        ))
    }

    if let Some(d) = read.playlists.get(&uuid) {
        res = Ok(Json(read::Payload::Playlist(vec![(uuid, d.clone()).into()])));
    } else {
        error!("[Api] No Playlist with {uuid} was found");
        return Err((StatusCode::BAD_REQUEST,
            Json((2, format!("No Playlist with the Uuid {uuid} was found")).into())
        ))
    }

    drop(read);

    store.delete_playlist(uuid).await;
    info!("[Api] Deleted Playlist {uuid}");
    res
}

async fn delete_schedule(State(state): State<AppState>, Path(uuid): Path<Uuid>) -> read::Response {
    info!("[Api] Deleting Schedule {uuid}");
    let res;
    let store = state.store;
    let read = store.read().await;

    let dependant_displays = read.displays.iter()
        .filter_map(|(_, d)|
            if d.schedule == uuid {
                Some(d.name.clone())
            } else {
                None
            }
        ).collect::<Vec<_>>();
    if dependant_displays.len() > 0 {
        return Err((StatusCode::BAD_REQUEST,
            Json((1, format!("Unable to delete playlist since the Displays ({}) depend on it",
                dependant_displays.join(", ")
            )).into())
        ))
    }

    if let Some(s) = read.schedules.get(&uuid) {
        res = Ok(Json(read::Payload::Schedule(vec![(uuid, s.clone()).into()])));
    } else {
        error!("[Api] No Schedule with {uuid} was found");
        return Err((StatusCode::BAD_REQUEST,
            Json((2, format!("No Schedule with the Uuid {uuid} was found")).into())
        ))
    }

    drop(read);

    store.delete_schedule(uuid).await;
    info!("[Api] Deleted Schedule {uuid}");
    res
}

async fn ws_handler(ws: WebSocketUpgrade, ConnectInfo(addr): ConnectInfo<SocketAddr>, State(state): State<AppState>) -> impl IntoResponse {
    ws.on_upgrade(move |socket| client_connection(socket, addr, state.store))
}
