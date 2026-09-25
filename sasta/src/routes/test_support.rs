use std::{env, sync::Arc};

use tokio::sync::{Mutex, broadcast};
use uuid::Uuid;

use crate::{AppState, file_server::file_server::FileServer, store::store::Store};

pub fn test_prefix(resource: &str) -> String {
    format!("test_sasta_{resource}_{}", Uuid::new_v4())
}

pub async fn cleanup_test_content(state: &AppState, prefix: &str) -> Result<(), String> {
    let displays = state
        .store
        .displays()
        .await
        .map_err(|error| error.to_string())?;
    for uuid in displays
        .into_iter()
        .filter(|(_, display)| display.name.starts_with(prefix))
        .map(|(uuid, _)| uuid)
    {
        state
            .store
            .delete_display(uuid)
            .await
            .map_err(|error| error.to_string())?;
    }

    let schedules = state
        .store
        .schedules()
        .await
        .map_err(|error| error.to_string())?;
    for uuid in schedules
        .into_iter()
        .filter(|(_, schedule)| schedule.name.starts_with(prefix))
        .map(|(uuid, _)| uuid)
    {
        state
            .store
            .delete_schedule(uuid)
            .await
            .map_err(|error| error.to_string())?;
    }

    let playlists = state
        .store
        .playlists()
        .await
        .map_err(|error| error.to_string())?;
    for uuid in playlists
        .into_iter()
        .filter(|(_, playlist)| playlist.name.starts_with(prefix))
        .map(|(uuid, _)| uuid)
    {
        state
            .store
            .delete_playlist(uuid)
            .await
            .map_err(|error| error.to_string())?;
    }

    Ok(())
}

pub async fn app_state() -> AppState {
    dotenvy::dotenv().ok();
    let redis_url = env::var("REDIS_URL").expect("REDIS_URL must point to Redis for route tests");
    let file_path = env::temp_dir().join(format!("sasta-route-tests-{}", Uuid::new_v4()));
    let file_server = FileServer::new("unused in tests", file_path).await;
    let store = Store::new(&redis_url).await;
    let (events, _) = broadcast::channel(16);

    AppState {
        store,
        file_server: Arc::new(Mutex::new(file_server)),
        events,
        htmx_hash: Arc::<str>::from(""),
    }
}
