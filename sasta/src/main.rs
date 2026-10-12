use std::{env, net::SocketAddr, str::FromStr, sync::Arc};

use axum::{
    Router,
    extract::{ConnectInfo, State, WebSocketUpgrade},
    response::IntoResponse,
    routing::get,
};
use tokio::{signal, sync::broadcast};
use tower_http::services::ServeDir;
use tracing::{debug, info, instrument, warn};
use tracing_subscriber::{EnvFilter, fmt::format::FmtSpan};
use utoipa::OpenApi;
use utoipa_axum::router::OpenApiRouter;
use utoipa_rapidoc::RapiDoc;
use utoipa_redoc::{Redoc, Servable};
use utoipa_swagger_ui::SwaggerUi;

use crate::{
    casta::casta::{casta_index, compute_hash, minify},
    connection::connection::client_connection,
    file_server::file_server::FileServer,
    routes::{
        display::{ReadDisplay, display_router},
        file_server::{ListView, file_api_router, get_file},
        playlist::{ReadPlaylist, playlist_router},
        schedule::{ReadSchedule, schedule_router},
    },
    store::store::{Change, Store, redis_event_listener},
};

mod casta;
mod connection;
mod file_server;
mod routes;
mod store;

#[derive(Clone)]
pub struct AppState {
    pub store: Store,
    pub file_server: FileServer,
    pub events: broadcast::Sender<Change>,
    pub htmx_hash: Arc<str>,
}

#[derive(OpenApi)]
#[openapi(
    info(
        title = "Sasta API",
        description = "REST API for **Sasta**, the backend server of the Asta display management system.\n\n### Core Concepts\n- **Displays (`/api/display`)**: Physical screens connected to Sasta. Each display is configured to show either a single **Playlist** continuously or a time-slotted **Schedule**.\n- **Schedules (`/api/schedule`)**: Time-based rules (using 7-field cron expressions) that switch between different playlists over time, with a default fallback playlist when no scheduled window is active.\n- **Playlists (`/api/playlist`)**: Ordered sequences of media items (`WEBSITE`, `TEXT`, `IMAGE`, `BACKGROUND_AUDIO`, `PDF`), each with a display duration in seconds.\n- **Files (`/api/files`)**: Virtual filesystem for uploading, organizing, renaming, and deleting static assets (such as images and PDFs) stored on the server. Uploaded files are served directly at `GET /files/{path}` and can be referenced in playlist items using the `ASTA://{path}` prefix."
    ),
    tags(
        (name = "display", description = "Manage displays and assign them a Schedule or Playlist."),
        (name = "schedule", description = "Manage time-slotted schedules that switch between playlists using 7-field cron expressions."),
        (name = "playlist", description = "Manage playlists and their ordered media items."),
        (name = "files", description = "Manage uploaded files and directories in the virtual filesystem. Uploaded files can be fetched directly at `GET /files/{path}`.")
    ),
    components(
        schemas(ReadDisplay, ReadSchedule, ReadPlaylist, ListView)
    )
)]
struct ApiDoc;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    let redis_url = env::var("REDIS_URL").expect("REDIS_URL variable must be set");
    let sasta_address = env::var("ADDRESS").unwrap_or("127.0.0.1:8080".into());
    let sasta_file_path = env::var("FILE_PATH").unwrap_or("./files".into());

    let (env_filter, filter_warning) = match env::var("RUST_LOG") {
        Ok(val) => match EnvFilter::try_new(&val) {
            Ok(filter) => {
                let filter = if val.contains("tower_http") {
                    filter
                } else {
                    filter.add_directive("tower_http=off".parse().unwrap())
                };
                (filter, None)
            }
            Err(err) => (
                EnvFilter::new("info,tower_http=off"),
                Some(format!(
                    "Couldn't parse RUST_LOG='{val}' ({err}), using default 'info'"
                )),
            ),
        },
        Err(_) => (EnvFilter::new("info,tower_http=off"), None),
    };

    tracing_subscriber::fmt()
        .with_env_filter(env_filter)
        .with_span_events(FmtSpan::CLOSE)
        .init();

    if let Some(warning) = filter_warning {
        warn!("{warning}");
    }

    debug!("REDIS_URL={redis_url}");
    debug!("ADDRESS={sasta_address}");
    debug!("FILE_PATH={sasta_file_path}");

    minify();
    let htmx_hash = Arc::<str>::from(compute_hash());
    let store = Store::new(&redis_url).await;
    let file_server = FileServer::new(&redis_url, sasta_file_path).await;

    let (events, _) = broadcast::channel(1024);
    tokio::spawn(redis_event_listener(store.client(), events.clone()));

    let app_state = AppState {
        store,
        file_server,
        events,
        htmx_hash,
    };

    let (api_router, openapi) = OpenApiRouter::with_openapi(ApiDoc::openapi())
        .nest(
            "/api",
            OpenApiRouter::new()
                .nest("/display", display_router())
                .nest("/schedule", schedule_router())
                .nest("/playlist", playlist_router())
                .nest("/files", file_api_router()),
        )
        .split_for_parts();

    let app = Router::new()
        .merge(api_router)
        .nest("/files", Router::new().fallback(get_file))
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", openapi.clone()))
        .merge(Redoc::with_url("/redoc", openapi.clone()))
        .merge(RapiDoc::with_openapi("/api-docs/openapi2.json", openapi).path("/rapidoc"))
        .route("/", get(ws_handler))
        .route("/ws", get(ws_handler))
        .route("/display/{uuid}", get(casta_index))
        .route("/casta/{uuid}", get(casta_index))
        .route("/ping", get(async || "pong"))
        .nest_service("/assets", ServeDir::new("assets"))
        .with_state(app_state);

    let addr = SocketAddr::from_str(&sasta_address).expect("Wrong address format");
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    info!("listening on http://{}", addr);

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal())
    .await
    .unwrap();
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
    tracing::info!("Signal received, Sasta shutting down");
}

#[instrument(level = "debug", skip(ws, state))]
async fn ws_handler(
    ws: WebSocketUpgrade,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| {
        client_connection(
            socket,
            addr,
            state.store,
            state.events.subscribe(),
            state.htmx_hash.to_string(),
        )
    })
}
