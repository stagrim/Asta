use std::{collections::HashSet, env, net::SocketAddr, str::FromStr, sync::Arc, vec};

use axum::{
    extract::{ConnectInfo, Path, State, WebSocketUpgrade},
    response::IntoResponse,
    routing::{delete, get, post, put},
    Json, Router,
};
use axum_macros::debug_handler;
use chrono::Local;
use hyper::StatusCode;
use read::Payload;
use store::{schedule::Moment, store::Store};
use tokio::sync::{oneshot, Mutex};
use tower_http::services::ServeDir;
use tracing::{error, info, info_span, Level};
use tracing_subscriber::{fmt::format::FmtSpan, FmtSubscriber};
use utoipa::OpenApi;
use utoipa_rapidoc::RapiDoc;
use utoipa_redoc::{Redoc, Servable};
use utoipa_swagger_ui::SwaggerUi;
use uuid::Uuid;

use crate::{
    casta::casta::{casta_index, compute_hash, minify},
    connection::connection::client_connection,
    file_server::file_server::{add_files, get_all_paths, get_file, FileServer},
    store::{schedule, store::DisplayMaterial},
};

mod casta;
mod connection;
mod file_server;
mod store;

impl From<(u8, String)> for read::Payload {
    fn from(value: (u8, String)) -> Self {
        read::Payload::Error {
            code: value.0,
            message: value.1,
        }
    }
}

// TODO: Race conditions possible in like all API routes. Use Mutex instead?

#[derive(Clone)]
pub struct AppState {
    store: Arc<Store>,
    file_server: Arc<Mutex<FileServer>>,
    htmx_hash: String,
}

#[derive(OpenApi)]
#[openapi(
    paths(
        read_display,
        read_schedule,
        read_playlist,
        // create_display,
        // create_schedule,
        // create_playlist,
        update_display,
        // update_schedule,
        // update_playlist,
        delete_display,
        delete_schedule,
        delete_playlist,
    ),
    components(
        schemas(read::Payload)
    ),
    modifiers(),
    tags(
        (name = "todo", description = "Todo items management API")
    )
)]
struct ApiDoc;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    let redis_url = env::var("REDIS_URL").expect("REDIS_URL variable must be set");
    let sasta_address = env::var("ADDRESS").unwrap_or("127.0.0.1:8080".into());
    let subscriber = FmtSubscriber::builder()
        .with_span_events(FmtSpan::NEW)
        .with_max_level(Level::TRACE)
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    minify();
    info!("JS and CSS minified");
    let htmx_hash = compute_hash();
    info!("Computed Hash for Casta Htmx");

    let store = Arc::new(Store::new(&redis_url).await);
    let file_server = Arc::new(Mutex::new(FileServer::new(&redis_url).await));

    let store_copy = store.clone();
    let (tx, rx) = oneshot::channel::<()>();

    tokio::spawn(async move {
        store_copy.schedule_loop(tx).await;
    });

    rx.await.unwrap();

    info!("{}", store.to_string().await);

    let app_state = AppState {
        store,
        file_server,
        htmx_hash,
    };

    let app = Router::new()
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))
        .merge(Redoc::with_url("/redoc", ApiDoc::openapi()))
        // There is no need to create `RapiDoc::with_openapi` because the OpenApi is served
        // via SwaggerUi instead we only make rapidoc to point to the existing doc.
        // .merge(RapiDoc::new("/api-docs/openapi.json").path("/rapidoc"))
        // Alternative to above
        .merge(RapiDoc::with_openapi("/api-docs/openapi2.json", ApiDoc::openapi()).path("/rapidoc"))
        .nest(
            "/api",
            Router::new()
                .nest(
                    "/display",
                    Router::new()
                        .route("/", get(read_display))
                        .route("/", post(create_display))
                        .route("/{uuid}", put(update_display))
                        .route("/{uuid}", delete(delete_display)),
                )
                .nest(
                    "/playlist",
                    Router::new()
                        .route("/", post(create_playlist))
                        .route("/", get(read_playlist))
                        .route("/{uuid}", put(update_playlist))
                        .route("/{uuid}", delete(delete_playlist)),
                )
                .nest(
                    "/schedule",
                    Router::new()
                        .route("/", post(create_schedule))
                        .route("/", get(read_schedule))
                        .route("/{uuid}", get(schedule_info))
                        .route("/{uuid}", put(update_schedule))
                        .route("/{uuid}", delete(delete_schedule)),
                )
                .nest(
                    "/files",
                    Router::new()
                        .route("/", get(get_all_paths))
                        .route("/", post(add_files)),
                ),
        )
        .nest("/files", Router::new().fallback(get_file))
        .route("/", get(ws_handler))
        .route("/ws", get(ws_handler))
        .route("/display/{uuid}", get(casta_index))
        .route("/casta/{uuid}", get(casta_index))
        .nest_service("/assets", ServeDir::new("assets"))
        .with_state(app_state);

    let addr = SocketAddr::from_str(&sasta_address).expect("Wrong address format");
    info!("listening on http://{}", addr);
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .unwrap();
}

mod read {
    use axum::Json;
    use hyper::StatusCode;
    use serde::Serialize;
    use ts_rs::TS;
    use utoipa::ToSchema;
    use uuid::Uuid;

    use crate::store::{
        schedule,
        store::{self, DisplayMaterial},
    };

    pub type Response = Result<Json<Payload>, (StatusCode, Json<Payload>)>;

    // https://github.com/juhaku/utoipa/issues/727
    #[derive(Serialize, ToSchema, TS)]
    #[serde(tag = "type", content = "content")]
    #[ts(export, export_to = "api_bindings/read/")]
    pub enum Payload {
        Display(Vec<Display>),
        Playlist(Vec<Playlist>),
        Schedule(Vec<Schedule>),
        Error { code: u8, message: String },
    }

    #[derive(Serialize, ToSchema, TS)]
    #[ts(export, export_to = "api_bindings/read/")]
    pub struct Display {
        #[ts(type = "string")]
        pub uuid: Uuid,
        pub name: String,
        pub display_material: DisplayMaterial,
    }

    impl From<(Uuid, store::Display)> for Display {
        fn from((uuid, d): (Uuid, store::Display)) -> Self {
            Self {
                uuid,
                name: d.name,
                display_material: d.display_material,
            }
        }
    }

    #[derive(Serialize, TS, ToSchema)]
    #[ts(export, export_to = "api_bindings/read/")]
    pub struct Playlist {
        #[ts(type = "string")]
        pub uuid: Uuid,
        pub name: String,
        pub items: Vec<store::PlaylistItem>,
    }

    impl From<(Uuid, store::Playlist)> for Playlist {
        fn from((uuid, p): (Uuid, store::Playlist)) -> Self {
            Self {
                uuid,
                name: p.name,
                items: p.items,
            }
        }
    }

    #[derive(Serialize, TS, ToSchema)]
    #[ts(export, export_to = "api_bindings/read/")]
    pub struct Schedule {
        #[ts(type = "string")]
        pub uuid: Uuid,
        pub name: String,
        #[ts(type = "string")]
        pub playlist: Uuid,
        pub scheduled: Option<Vec<schedule::ScheduledPlaylistInput>>,
    }

    impl From<(Uuid, schedule::Schedule)> for Schedule {
        fn from((uuid, s): (Uuid, schedule::Schedule)) -> Self {
            let s = schedule::ScheduleInput::from(s);
            Self {
                uuid,
                name: s.name,
                playlist: s.playlist,
                scheduled: s.scheduled,
            }
        }
    }

    #[derive(Serialize, TS)]
    #[ts(export, export_to = "api_bindings/read/")]
    pub struct ScheduleInfo {
        #[ts(type = "string")]
        pub current: Uuid,
        pub next: Option<NextMoment>,
    }

    #[derive(Serialize, TS)]
    #[ts(export, export_to = "api_bindings/read/")]
    pub struct NextMoment {
        /// Amount of milliseconds until change
        pub in_ms: u64,
        #[ts(type = "string")]
        pub playlist: Uuid,
    }
}

mod create {
    use serde::Deserialize;
    use ts_rs::TS;
    use uuid::Uuid;

    pub use crate::read::Response;
    use crate::store::store::DisplayMaterial;

    #[derive(Debug, Deserialize, TS)]
    #[ts(export, export_to = "api_bindings/create/", rename = "CreateDisplay")]
    pub struct Display {
        #[ts(type = "string", optional)]
        pub uuid: Option<Uuid>,
        pub name: String,
        pub display_material: DisplayMaterial,
    }

    #[derive(Deserialize, TS)]
    #[ts(export, export_to = "api_bindings/create/", rename = "CreatePlaylist")]
    pub struct Playlist {
        pub name: String,
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
    use utoipa::ToSchema;
    use uuid::Uuid;

    pub use crate::read::{Payload, Response};
    use crate::store::{
        schedule,
        store::{self, DisplayMaterial},
    };

    #[derive(Deserialize, ToSchema, TS)]
    #[ts(export, export_to = "api_bindings/update/", rename = "UpdateDisplay")]
    #[schema(title = "UpdateDisplay")]
    pub struct Display {
        pub name: String,
        pub display_material: DisplayMaterial,
    }

    #[derive(Deserialize, ToSchema, TS)]
    #[ts(export, export_to = "api_bindings/update/", rename = "UpdatePlaylist")]
    #[schema(title = "UpdatePlaylist")]
    pub struct Playlist {
        pub name: String,
        pub items: Vec<store::PlaylistItem>,
    }

    #[derive(Deserialize, ToSchema, TS)]
    #[ts(export, export_to = "api_bindings/update/", rename = "UpdateSchedule")]
    #[schema(title = "UpdateSchedule")]
    pub struct Schedule {
        pub name: String,
        #[ts(type = "string")]
        pub playlist: Uuid,
        #[ts(optional)]
        pub scheduled: Option<Vec<schedule::ScheduledPlaylistInput>>,
    }
}

#[debug_handler]
async fn create_display(
    State(state): State<AppState>,
    Json(disp): Json<create::Display>,
) -> read::Response {
    info_span!("[Api] Creating Display", display = ?disp);
    let store = state.store;
    let read = store.read().await;
    if let Some((uuid, _)) = read.displays.iter().find(|(_, d)| d.name == disp.name) {
        error!("[Api] Name is already used by Display {}", uuid);
        return Err((
            StatusCode::BAD_REQUEST,
            Json(
                (
                    1,
                    format!(
                        "Avoid using the name {} as it is already used by another display",
                        disp.name
                    ),
                )
                    .into(),
            ),
        ));
    }

    if let Some(uuid) = disp.uuid {
        if read.displays.contains_key(&uuid) {
            error!("[Api] Uuid is already used by another Display");
            return Err((
                StatusCode::BAD_REQUEST,
                Json(
                    (
                        2,
                        format!(
                            "Avoid using the Uuid {} as it is already used by another display",
                            disp.name
                        ),
                    )
                        .into(),
                ),
            ));
        }
    }
    drop(read);

    let uuid = match disp.uuid {
        Some(u) => u,
        None => Uuid::new_v4(),
    };
    info!("[Api] Using Uuid {uuid} for new Display");

    if let Err(e) = store
        .create_display(uuid, disp.name, disp.display_material)
        .await
    {
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json((5, format!("Could not write changes to db ({e})")).into()),
        ));
    }

    return if let Some(d) = store.read().await.displays.get(&uuid) {
        info!("[Api] Created Display {uuid}");
        Ok(Json(read::Payload::Display(vec![(uuid, d.clone()).into()])))
    } else {
        error!(
            "[Api] No Display with {} could be found while reading after write",
            uuid
        );
        Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json((3, format!("Something went wrong with the creation")).into()),
        ))
    };
}

async fn create_playlist(
    State(state): State<AppState>,
    Json(playlist): Json<create::Playlist>,
) -> create::Response {
    info!("[Api] Creating Playlist with name {}", playlist.name);
    let store = state.store;
    let read = store.read().await;
    if let Some((uuid, _)) = read.playlists.iter().find(|(_, p)| p.name == playlist.name) {
        error!("[Api] Name is already used by Playlist {}", uuid);
        return Err((
            StatusCode::BAD_REQUEST,
            Json(
                (
                    1,
                    format!(
                        "Avoid using the name {} as it is already used by another Playlist",
                        playlist.name
                    ),
                )
                    .into(),
            ),
        ));
    }

    drop(read);
    let uuid = Uuid::new_v4();
    info!("[Api] Generated Uuid {uuid} for new Playlist");

    if let Err(e) = store.create_playlist(uuid, playlist.name).await {
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json((5, format!("Could not write changes to db ({e})")).into()),
        ));
    }

    return if let Some(p) = store.read().await.playlists.get(&uuid) {
        info!("[Api] Created Playlist {uuid}");
        Ok(Json(read::Payload::Playlist(
            vec![(uuid, p.clone()).into()],
        )))
    } else {
        error!("[Api] No Playlist with {uuid} could be found while reading after write");
        Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json((2, format!("Something went wrong with the creation")).into()),
        ))
    };
}

async fn create_schedule(
    State(state): State<AppState>,
    Json(schedule): Json<create::Schedule>,
) -> create::Response {
    info!("[Api] Creating Schedule with name {}", schedule.name);
    let store = state.store;
    let read = store.read().await;
    if let Some((uuid, _)) = read.schedules.iter().find(|(_, s)| s.name == schedule.name) {
        error!("[Api] Name is already used by Schedule {}", uuid);
        return Err((
            StatusCode::BAD_REQUEST,
            Json(
                (
                    1,
                    format!(
                        "Avoid using the name {} as it is already used by another Schedule",
                        schedule.name
                    ),
                )
                    .into(),
            ),
        ));
    }

    drop(read);
    let uuid = Uuid::new_v4();
    info!("[Api] Generated Uuid {uuid} for new Schedule");

    if let Err(e) = store
        .create_schedule(uuid, schedule.name, schedule.playlist)
        .await
    {
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json((5, format!("Could not write changes to db ({e})")).into()),
        ));
    }

    return if let Some(s) = store.read().await.schedules.get(&uuid) {
        info!("[Api] Created Schedule {uuid}");
        Ok(Json(read::Payload::Schedule(
            vec![(uuid, s.clone()).into()],
        )))
    } else {
        error!("[Api] No Schedule with {uuid} could be found while reading after write");
        Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json((2, format!("Something went wrong with the creation")).into()),
        ))
    };
}

#[utoipa::path(
    get,
    path = "/api/display",
    tag = "display",
    responses(
        (status = 200, description = "Get all Displays", body = inline(read::Payload),
            example = json!(
                read::Payload::Display(vec![
                        read::Display { uuid: Uuid::new_v4(), name: "name1".into(), display_material: DisplayMaterial::Schedule(Uuid::new_v4()) },
                        read::Display { uuid: Uuid::new_v4(), name: "name2".into(), display_material: DisplayMaterial::Playlist(Uuid::new_v4()) },
                        read::Display { uuid: Uuid::new_v4(), name: "name3".into(), display_material: DisplayMaterial::Schedule(Uuid::new_v4()) }
                ])
            )
        ),
    )
)]
async fn read_display(State(state): State<AppState>) -> read::Response {
    return Ok(Json(read::Payload::Display(
        state
            .store
            .read()
            .await
            .displays
            .iter()
            .map(|(u, d)| (*u, d.clone()).into())
            .collect(),
    )));
}

#[utoipa::path(
    get,
    path = "/api/playlist",
    tag = "playlist",
    responses(
        (status = 200, description = "Get all Playlists", body = inline(read::Payload),
            example = json!(
                read::Payload::Playlist(vec![
                        read::Playlist { uuid: Uuid::new_v4(), name: "name1".into(), items: vec![
                            store::store::PlaylistItem::Website {
                                id: "item_name".into(),
                                settings: store::store::WebsiteData {
                                    url: "example.com".into(),
                                    duration: 60u64
                                }
                            }
                        ] },
                        read::Playlist { uuid: Uuid::new_v4(), name: "name2".into(), items: vec![] }
                ])
            )
        ),
    )
)]
async fn read_playlist(State(state): State<AppState>) -> read::Response {
    return Ok(Json(read::Payload::Playlist(
        state
            .store
            .read()
            .await
            .playlists
            .iter()
            .map(|(u, p)| (*u, p.clone()).into())
            .collect::<Vec<_>>(),
    )));
}

#[utoipa::path(
    get,
    path = "/api/schedule",
    tag = "schedule",
    responses(
        (status = 200, description = "Get all Schedules", body = inline(read::Payload),
            example = json!(
                read::Payload::Schedule(vec![
                    read::Schedule { uuid: Uuid::new_v4(), name: "name1".into(), playlist: Uuid::new_v4(), scheduled: Some(vec![
                        schedule::ScheduledPlaylistInput {
                            playlist: Uuid::new_v4(),
                            start: "0 0 10 * * Mon-Fri *".into(),
                            end: "0 0 14 * * Mon-Fri *".into()
                        }
                    ]) },
                    read::Schedule { uuid: Uuid::new_v4(), name: "name2".into(), playlist: Uuid::new_v4(), scheduled: Some(vec![]) }
                ])
            )
        ),
    )
)]
async fn read_schedule(State(state): State<AppState>) -> read::Response {
    return Ok(Json(read::Payload::Schedule(
        state
            .store
            .read()
            .await
            .schedules
            .iter()
            .map(|(u, s)| (*u, s.clone()).into())
            .collect::<Vec<_>>(),
    )));
}

#[utoipa::path(
    put,
    path = "/api/display/{uuid}",
    tag = "display",
    request_body(content = inline(update::Display)),
    responses(
        (status = 200, description = "Display updated", body = inline(read::Payload),
            example = json!(
                read::Payload::Display(vec![
                        read::Display { uuid: Uuid::new_v4(), name: "name".into(), display_material: DisplayMaterial::Schedule(Uuid::new_v4()) }
                ])
            )
        ),
        (status = BAD_REQUEST, body = Payload, examples(
            ("error_1" = (
                summary = "No Display found with given Uuid",
                value = json!(
                    read::Payload::from((1, format!("No Display with the Uuid <uuid> was found")))
                )
            )),
            ("error_2" = (
                summary = "Name is already used by another Display",
                value = json!(
                    read::Payload::from((2, format!("Avoid using the name <name> as it is already used by another display")))
                )
            ))
        )),
    ),
    params(
        ("uuid" = Uuid, Path, description = "Uuid of Display to delete")
    )
)]
async fn update_display(
    State(state): State<AppState>,
    Path(uuid): Path<Uuid>,
    Json(display): Json<update::Display>,
) -> update::Response {
    info!("[Api] Updating Display {uuid}");
    let store = state.store;
    let read = store.read().await;
    if !read.displays.contains_key(&uuid) {
        error!("[Api] No display with {uuid} was found");
        return Err((
            StatusCode::BAD_REQUEST,
            Json((1, format!("No Display with the Uuid {uuid} was found")).into()),
        ));
    }
    if let Some((uuid, _)) = read
        .displays
        .iter()
        .find(|(&u, d)| d.name == display.name && u != uuid)
    {
        error!("[Api] Name is already used by Display {}", uuid);
        return Err((
            StatusCode::BAD_REQUEST,
            Json(
                (
                    2,
                    format!(
                        "Avoid using the name {} as it is already used by another display",
                        display.name
                    ),
                )
                    .into(),
            ),
        ));
    }
    drop(read);

    if let Err(e) = store
        .update_display(uuid, display.name, display.display_material)
        .await
    {
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json((5, format!("Could not write changes to db ({e})")).into()),
        ));
    }

    return if let Some(d) = store.read().await.displays.get(&uuid) {
        info!("[Api] Updated and read Display {uuid}");
        Ok(Json(update::Payload::Display(vec![
            (uuid, d.clone()).into()
        ])))
    } else {
        error!("[Api] Could not find Display with {uuid} after update");
        Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(
                (
                    3,
                    format!("Could not find Display with {uuid} after update"),
                )
                    .into(),
            ),
        ))
    };
}

async fn update_playlist(
    State(state): State<AppState>,
    Path(uuid): Path<Uuid>,
    Json(playlist): Json<update::Playlist>,
) -> update::Response {
    info!("[Api] Updating Playlist {uuid}");
    let store = state.store;
    let read = store.read().await;
    if !read.playlists.contains_key(&uuid) {
        error!("[Api] No Playlist with {uuid} was found");
        return Err((
            StatusCode::BAD_REQUEST,
            Json((1, format!("No Playlist with the Uuid {uuid} was found")).into()),
        ));
    }
    if let Some((uuid, _)) = read
        .playlists
        .iter()
        .find(|(&u, p)| p.name == playlist.name && u != uuid)
    {
        error!("[Api] Name is already used by Playlist {}", uuid);
        return Err((
            StatusCode::BAD_REQUEST,
            Json(
                (
                    2,
                    format!(
                        "Avoid using the name {} as it is already used by another Playlist",
                        playlist.name
                    ),
                )
                    .into(),
            ),
        ));
    }

    drop(read);

    if let Err(e) = store
        .update_playlist(uuid, playlist.name, playlist.items)
        .await
    {
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json((5, format!("Could not write changes to db ({e})")).into()),
        ));
    }

    return if let Some(p) = store.read().await.playlists.get(&uuid) {
        info!("[Api] Updated and read Playlist {uuid}");
        Ok(Json(update::Payload::Playlist(vec![
            (uuid, p.clone()).into()
        ])))
    } else {
        error!("[Api] Could not find Playlist with {uuid} after update");
        Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(
                (
                    4,
                    format!("Could not find Playlist with {uuid} after update"),
                )
                    .into(),
            ),
        ))
    };
}

async fn update_schedule(
    State(state): State<AppState>,
    Path(uuid): Path<Uuid>,
    Json(schedule): Json<update::Schedule>,
) -> update::Response {
    info!("[Api] Updating Schedule {uuid}");
    let store = state.store;
    let read = store.read().await;
    if !read.schedules.contains_key(&uuid) {
        error!("[Api] No Schedule with {uuid} was found");
        return Err((
            StatusCode::BAD_REQUEST,
            Json((1, format!("No Schedule with the Uuid {uuid} was found")).into()),
        ));
    }
    if let Some((uuid, _)) = read
        .schedules
        .iter()
        .find(|(&u, s)| s.name == schedule.name && u != uuid)
    {
        error!("[Api] Name is already used by Schedule {}", uuid);
        return Err((
            StatusCode::BAD_REQUEST,
            Json(
                (
                    2,
                    format!(
                        "Avoid using the name {} as it is already used by another Schedule",
                        schedule.name
                    ),
                )
                    .into(),
            ),
        ));
    }

    if let Some(scheduled) = &schedule.scheduled {
        let mut uniq = HashSet::new();
        uniq.insert(schedule.playlist);
        // Checks if any playlist Uuid is a duplicate
        if !scheduled.iter().all(|s| uniq.insert(s.playlist)) {
            error!("[Api] Schedule contains duplicate Playlists");
            return Err((StatusCode::BAD_REQUEST, Json((3, format!("Must not use the same Playlist more than once in a Schedule to avoid server meltdown")).into())));
        }
    }
    drop(read);

    if let Err(e) = store
        .update_schedule(
            uuid,
            schedule.name,
            schedule.playlist,
            schedule.scheduled.unwrap_or(vec![]),
        )
        .await
    {
        error!("[Api] Schedule update failed with error: {e}");
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json((4, format!("{e}")).into()),
        ));
    }

    return if let Some(s) = store.read().await.schedules.get(&uuid) {
        info!("[Api] Updated and read Schedule {uuid}");
        Ok(Json(update::Payload::Schedule(vec![
            (uuid, s.clone()).into()
        ])))
    } else {
        error!("[Api] Could not find Schedule with {uuid} after update");
        Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(
                (
                    5,
                    format!(
                        "Could not find Schedule with {uuid} after update to avoid server meltdown"
                    ),
                )
                    .into(),
            ),
        ))
    };
}

#[utoipa::path(
    delete,
    path = "/api/display/{uuid}",
    tag = "display",
    responses(
        (status = 200, description = "Display deleted", body = Payload,
            example = json!(
                read::Payload::Display(vec![
                        read::Display { uuid: Uuid::new_v4(), name: "name".into(), display_material: DisplayMaterial::Schedule(Uuid::new_v4()) }
                ])
            )
        ),
        (status = BAD_REQUEST, body = Payload,
            description = "No Display exists with given Uuid",
            example = json!(
                read::Payload::from((1, format!("No Display with the Uuid <uuid> was found")))
            )
        )
    ),
    params(
        ("uuid" = Uuid, Path, description = "Uuid of Display to delete")
    )
)]
async fn delete_display(State(state): State<AppState>, Path(uuid): Path<Uuid>) -> read::Response {
    info!("[Api] Deleting Display {uuid}");
    let store = state.store;
    let res;
    if let Some(d) = store.read().await.displays.get(&uuid) {
        res = Ok(Json(read::Payload::Display(vec![(uuid, d.clone()).into()])));
    } else {
        error!("[Api] No display with {uuid} was found");
        return Err((
            StatusCode::BAD_REQUEST,
            Json((1, format!("No Display with the Uuid {uuid} was found")).into()),
        ));
    }

    if let Err(e) = store.delete_display(uuid).await {
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json((5, format!("Could not write changes to db ({e})")).into()),
        ));
    }

    info!("[Api] Deleted Display {uuid}");
    res
}

#[utoipa::path(
    delete,
    path = "/api/playlist/{uuid}",
    tag = "playlist",
    responses(
        (status = 200, description = "Playlist deleted", body = Payload,
            example = json!(
                read::Payload::Playlist(vec![
                        read::Playlist { uuid: Uuid::new_v4(), name: "name".into(), items: vec![] }
                ])
            )
        ),
        (status = BAD_REQUEST, body = Payload, examples(
            ("error_1" = (
                summary = "Playlist(s) depend on Playlist",
                value = json!(
                    read::Payload::from((1, format!("Unable to delete playlist since the Schedules (<schedules>) depend on it")))
                )
            )),
            ("error_2" = (
                summary = "No playlist exists with given Uuid",
                value = json!(
                    read::Payload::from((2, format!("No Playlist with the Uuid <uuid> was found")))
                )
            ))
        ))
    ),
    params(
        ("uuid" = Uuid, Path, description = "Uuid of Playlist to delete")
    )
)]
pub(crate) async fn delete_playlist(
    State(state): State<AppState>,
    Path(uuid): Path<Uuid>,
) -> read::Response {
    info!("[Api] Deleting Playlist {uuid}");
    let res;
    let store = state.store;
    let read = store.read().await;

    let dependant_schedules = read
        .schedules
        .iter()
        .filter_map(|(_, s)| {
            if s.all_playlists().iter().any(|&p| p == &uuid) {
                Some(s.name.clone())
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    if dependant_schedules.len() > 0 {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(
                (
                    1,
                    format!(
                        "Unable to delete playlist since the Schedules ({}) depend on it",
                        dependant_schedules.join(", ")
                    ),
                )
                    .into(),
            ),
        ));
    }

    let dependant_displays = read
        .displays
        .iter()
        .filter_map(|(_, d)| match d.display_material {
            DisplayMaterial::Playlist(playlist_uuid) if uuid == playlist_uuid => {
                Some(d.name.clone())
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    if dependant_displays.len() > 0 {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(
                (
                    2,
                    format!(
                        "Unable to delete playlist since the Displays ({}) depend on it",
                        dependant_displays.join(", ")
                    ),
                )
                    .into(),
            ),
        ));
    }

    if let Some(d) = read.playlists.get(&uuid) {
        res = Ok(Json(read::Payload::Playlist(
            vec![(uuid, d.clone()).into()],
        )));
    } else {
        error!("[Api] No Playlist with {uuid} was found");
        return Err((
            StatusCode::BAD_REQUEST,
            Json((3, format!("No Playlist with the Uuid {uuid} was found")).into()),
        ));
    }

    drop(read);

    if let Err(e) = store.delete_playlist(uuid).await {
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json((5, format!("Could not write changes to db ({e})")).into()),
        ));
    }

    info!("[Api] Deleted Playlist {uuid}");
    res
}

#[utoipa::path(
    delete,
    path = "/api/schedule/{uuid}",
    tag = "schedule",
    responses(
        (status = 200, description = "Schedule deleted", body = Payload,
            example = json!(
                read::Payload::Schedule(vec![
                        read::Schedule { uuid: Uuid::new_v4(), name: "name".into(), playlist: Uuid::new_v4(), scheduled: None }
                ])
            )
        ),
        (status = BAD_REQUEST, body = Payload, examples(
            ("error_1" = (
                summary = "Display(s) depend on Schedule",
                value = json!(
                    read::Payload::from((1, format!("Unable to delete Schedule since the Displays (<displays>) depend on it")))
                )
            )),
            ("error_2" = (
                summary = "No Schedule exists with given Uuid",
                value = json!(
                    read::Payload::from((2, format!("No Schedule with the Uuid <uuid> was found")))
                )
            ))
        ))
    ),
    params(
        ("uuid" = Uuid, Path, description = "Uuid of Schedule to delete")
    )
)]
async fn delete_schedule(State(state): State<AppState>, Path(uuid): Path<Uuid>) -> read::Response {
    info!("[Api] Deleting Schedule {uuid}");
    let res;
    let store = state.store;
    let read = store.read().await;

    let dependant_displays = read
        .displays
        .iter()
        .filter_map(|(_, d)| match d.display_material {
            DisplayMaterial::Schedule(schedule_uuid) if uuid == schedule_uuid => {
                Some(d.name.clone())
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    if dependant_displays.len() > 0 {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(
                (
                    1,
                    format!(
                        "Unable to delete playlist since the Displays ({}) depend on it",
                        dependant_displays.join(", ")
                    ),
                )
                    .into(),
            ),
        ));
    }

    if let Some(s) = read.schedules.get(&uuid) {
        res = Ok(Json(read::Payload::Schedule(
            vec![(uuid, s.clone()).into()],
        )));
    } else {
        error!("[Api] No Schedule with {uuid} was found");
        return Err((
            StatusCode::BAD_REQUEST,
            Json((2, format!("No Schedule with the Uuid {uuid} was found")).into()),
        ));
    }

    drop(read);

    if let Err(e) = store.delete_schedule(uuid).await {
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json((5, format!("Could not write changes to db ({e})")).into()),
        ));
    }

    info!("[Api] Deleted Schedule {uuid}");
    res
}

async fn schedule_info(State(state): State<AppState>, Path(uuid): Path<Uuid>) -> impl IntoResponse {
    let current_moment = Local::now();
    let store = state.store.read().await;

    if let Some(schedule) = store.schedules.get(&uuid) {
        let next_moment =
            schedule
                .next_schedule(&current_moment)
                .and_then(|Moment { time, playlist }| {
                    Some(read::NextMoment {
                        in_ms: (time - current_moment).num_milliseconds() as u64,
                        playlist,
                    })
                });

        Ok(Json(read::ScheduleInfo {
            current: schedule.current_playlist(&current_moment),
            next: next_moment,
        }))
    } else {
        Err(format!("Schedule '{uuid}' not found"))
    }
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| client_connection(socket, addr, state.store, state.htmx_hash))
}
