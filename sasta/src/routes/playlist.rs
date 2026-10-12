use axum::{
    Json,
    extract::{Path, State},
};
use hyper::StatusCode;
use serde::{Deserialize, Serialize};
use tracing::{error, instrument};
use utoipa::ToSchema;
use utoipa_axum::{router::OpenApiRouter, routes};
use uuid::Uuid;

use crate::{
    AppState,
    store::store::{self, Playlist},
};

/// Request payload for creating a new (initially empty) playlist.
#[derive(Debug, Deserialize, ToSchema)]
#[schema(example = json!({
    "name": "Default Daytime Playlist"
}))]
pub struct CreatePlaylist {
    /// Unique human-readable name for the playlist. Must not match any existing playlist's name.
    #[schema(example = "Default Daytime Playlist")]
    pub name: String,
}

/// Representation of a playlist returned by the API.
#[derive(Serialize, ToSchema)]
#[schema(example = json!({
    "uuid": "660e8400-e29b-41d4-a716-446655440000",
    "name": "Default Daytime Playlist",
    "items": [
        {
            "type": "WEBSITE",
            "id": "guild-website",
            "settings": {
                "url": "https://dsek.se",
                "duration": 60
            }
        },
        {
            "type": "IMAGE",
            "id": "welcome-poster",
            "settings": {
                "src": "ASTA://posters/welcome.png",
                "duration": 30
            }
        }
    ]
}))]
pub struct ReadPlaylist {
    /// Unique identifier of the playlist.
    pub uuid: Uuid,
    /// Unique human-readable name of the playlist.
    #[schema(example = "Default Daytime Playlist")]
    pub name: String,
    /// Ordered list of media items cycled through when this playlist is active.
    pub items: Vec<store::PlaylistItem>,
}
impl From<(Uuid, Playlist)> for ReadPlaylist {
    fn from((uuid, p): (Uuid, Playlist)) -> Self {
        Self {
            uuid,
            name: p.name,
            items: p.items,
        }
    }
}

/// Request payload for updating a playlist's name and replacing its ordered list of media items.
#[derive(Debug, Deserialize, ToSchema)]
#[schema(example = json!({
    "name": "Default Daytime Playlist",
    "items": [
        {
            "type": "TEXT",
            "id": "announcement",
            "settings": {
                "text": "Lunch lecture today at 12:15!",
                "duration": 30
            }
        }
    ]
}))]
pub struct UpdatePlaylist {
    /// Updated name for the playlist. Must not conflict with any other playlist's name.
    #[schema(example = "Default Daytime Playlist")]
    pub name: String,
    /// Complete ordered list of media items for the playlist.
    pub items: Vec<store::PlaylistItem>,
}
type Response = Result<Json<ReadPlaylist>, (StatusCode, String)>;

#[utoipa::path(
    post,
    path = "/",
    tag = "playlist",
    summary = "Create a new playlist",
    description = "Creates a new playlist with the given `name` and an empty `items` array. To add or reorder media items in the playlist, call `PUT /api/playlist/{uuid}`.",
    request_body(content = CreatePlaylist),
    responses(
        (status = 200, description = "Playlist created", body = ReadPlaylist),
        (status = 400, description = "Bad Request (playlist name is already taken)", body = String,
            example = json!("Avoid using the name Default Daytime Playlist as it is already used by another Playlist")
        ),
        (status = 500, description = "Server Error", body = String)
    )
)]
#[instrument(level = "debug", skip(state))]
async fn create_playlist(
    State(state): State<AppState>,
    Json(input): Json<CreatePlaylist>,
) -> Response {
    let playlists = state.store.playlists().await.map_err(internal)?;
    if playlists.values().any(|p| p.name == input.name) {
        return Err((
            StatusCode::BAD_REQUEST,
            format!(
                "Avoid using the name {} as it is already used by another Playlist",
                input.name
            ),
        ));
    }
    let uuid = Uuid::new_v4();
    let playlist = Playlist {
        name: input.name,
        items: vec![],
    };
    state
        .store
        .create_playlist(uuid, &playlist)
        .await
        .map_err(internal)?;
    Ok(Json((uuid, playlist).into()))
}

#[utoipa::path(
    get,
    path = "/",
    tag = "playlist",
    summary = "List all playlists",
    description = "Returns all playlists and their ordered media items.",
    responses(
        (status = 200, description = "Get all Playlists", body = inline(Vec<ReadPlaylist>),
            example = json!(
                vec![
                    ReadPlaylist { uuid: Uuid::new_v4(), name: "name1".into(), items: vec![
                        store::PlaylistItem::Website {
                            id: "item_name".into(),
                            settings: store::WebsiteData {
                                url: "example.com".into(),
                                duration: 60u64
                            }
                        }
                    ] },
                    ReadPlaylist { uuid: Uuid::new_v4(), name: "name2".into(), items: vec![] }
                ]
            )
        ),
        (status = 500, description = "Server Error", body = String)
    )
)]
#[instrument(level = "debug", skip(state))]
async fn read_playlist(
    State(state): State<AppState>,
) -> Result<Json<Vec<ReadPlaylist>>, (StatusCode, String)> {
    let playlists = state.store.playlists().await.map_err(internal)?;
    Ok(Json(playlists.into_iter().map(Into::into).collect()))
}

#[utoipa::path(
    put,
    path = "/{uuid}",
    tag = "playlist",
    summary = "Update a playlist",
    description = "Updates the name and replaces the `items` list of an existing playlist. Any displays currently showing this playlist (either directly or via an active schedule) are notified immediately.",
    request_body(content = UpdatePlaylist),
    responses(
        (status = 200, description = "Playlist updated", body = ReadPlaylist),
        (status = 400, description = "Bad Request (UUID not found or name already used)", body = String, examples(
            ("not_found" = (
                summary = "No Playlist found with given Uuid",
                value = json!("No Playlist with the Uuid <uuid> was found")
            )),
            ("name_taken" = (
                summary = "Name is already used by another Playlist",
                value = json!("Avoid using the name <name> as it is already used by another Playlist")
            ))
        )),
        (status = 500, description = "Server Error", body = String)
    ),
    params(
        ("uuid" = Uuid, Path, description = "Uuid of Playlist to update")
    )
)]
#[instrument(level = "debug", skip(state))]
async fn update_playlist(
    State(state): State<AppState>,
    Path(uuid): Path<Uuid>,
    Json(input): Json<UpdatePlaylist>,
) -> Response {
    let playlists = state.store.playlists().await.map_err(internal)?;
    if !playlists.contains_key(&uuid) {
        return Err((
            StatusCode::BAD_REQUEST,
            format!("No Playlist with the Uuid {uuid} was found"),
        ));
    }
    if playlists
        .iter()
        .any(|(id, p)| *id != uuid && p.name == input.name)
    {
        return Err((
            StatusCode::BAD_REQUEST,
            format!(
                "Avoid using the name {} as it is already used by another Playlist",
                input.name
            ),
        ));
    }
    let playlist = Playlist {
        name: input.name,
        items: input.items,
    };
    state
        .store
        .update_playlist(uuid, &playlist)
        .await
        .map_err(internal)?;
    Ok(Json((uuid, playlist).into()))
}

#[utoipa::path(
    delete,
    path = "/{uuid}",
    tag = "playlist",
    summary = "Delete a playlist",
    description = "Deletes the playlist with the specified UUID. Deletion is rejected with `400 Bad Request` if any Schedule (either as fallback or scheduled item) or any Display currently references this playlist.",
    responses(
        (status = 200, description = "Playlist deleted", body = ReadPlaylist,
            example = json!(
                ReadPlaylist { uuid: Uuid::new_v4(), name: "name".into(), items: vec![] }
            )
        ),
        (status = BAD_REQUEST, description = "Playlist not found or still referenced by a Schedule or Display", body = String, examples(
            ("error_1" = (
                summary = "A Schedule depends on this Playlist",
                value = json!("Unable to delete playlist since a Schedule depends on it")
            )),
            ("error_2" = (
                summary = "A Display depends on this Playlist",
                value = json!("Unable to delete playlist since a Display depends on it")
            )),
            ("error_3" = (
                summary = "No playlist exists with given Uuid",
                value = json!(
                    format!("No Playlist with the Uuid <uuid> was found")
                )
            ))
        )),
        (status = 500, description = "Server Error", body = String)
    ),
    params(
        ("uuid" = Uuid, Path, description = "Uuid of Playlist to delete")
    )
)]
#[instrument(level = "debug", skip(state))]
async fn delete_playlist(State(state): State<AppState>, Path(uuid): Path<Uuid>) -> Response {
    let playlist = state
        .store
        .playlist(uuid)
        .await
        .map_err(internal)?
        .ok_or_else(|| {
            (
                StatusCode::BAD_REQUEST,
                format!("No Playlist with the Uuid {uuid} was found"),
            )
        })?;
    let schedules = state.store.schedules().await.map_err(internal)?;
    if schedules
        .values()
        .any(|s| s.all_playlists().iter().any(|id| **id == uuid))
    {
        return Err((
            StatusCode::BAD_REQUEST,
            "Unable to delete playlist since a Schedule depends on it".into(),
        ));
    }
    let displays = state.store.displays().await.map_err(internal)?;
    if displays
        .values()
        .any(|d| matches!(d.display_material, store::DisplayMaterial::Playlist(id) if id == uuid))
    {
        return Err((
            StatusCode::BAD_REQUEST,
            "Unable to delete playlist since a Display depends on it".into(),
        ));
    }
    state.store.delete_playlist(uuid).await.map_err(internal)?;
    Ok(Json((uuid, playlist).into()))
}

fn internal<E: std::fmt::Display>(e: E) -> (StatusCode, String) {
    error!("Redis error: {e}");
    (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
}
pub fn playlist_router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(create_playlist))
        .routes(routes!(read_playlist))
        .routes(routes!(update_playlist))
        .routes(routes!(delete_playlist))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::routes::test_support::{cleanup_test_content, test_prefix};

    #[tokio::test]
    async fn playlist_routes_support_crud() {
        let state = crate::routes::test_support::app_state().await;
        let prefix = test_prefix("playlist");
        let initial_name = format!("{prefix}_initial");
        let updated_name = format!("{prefix}_updated");

        let result: Result<(), String> = async {
            let created = create_playlist(
                State(state.clone()),
                Json(CreatePlaylist {
                    name: initial_name.clone(),
                }),
            )
            .await
            .map_err(|(status, message)| format!("{status}: {message}"))?
            .0;
            let uuid = created.uuid;
            if created.name != initial_name || !created.items.is_empty() {
                return Err("create returned unexpected playlist data".into());
            }

            let listed = read_playlist(State(state.clone()))
                .await
                .map_err(|(status, message)| format!("{status}: {message}"))?
                .0;
            if !listed.iter().any(|playlist| playlist.uuid == uuid) {
                return Err("created playlist was missing from list".into());
            }

            let updated = update_playlist(
                State(state.clone()),
                Path(uuid),
                Json(UpdatePlaylist {
                    name: updated_name.clone(),
                    items: vec![],
                }),
            )
            .await
            .map_err(|(status, message)| format!("{status}: {message}"))?
            .0;
            if updated.name != updated_name {
                return Err("update returned unexpected playlist data".into());
            }

            let listed = read_playlist(State(state.clone()))
                .await
                .map_err(|(status, message)| format!("{status}: {message}"))?
                .0;
            let persisted = listed
                .iter()
                .find(|playlist| playlist.uuid == uuid)
                .ok_or_else(|| "updated playlist was missing from list".to_string())?;
            if persisted.name != updated_name {
                return Err("updated playlist was not persisted".into());
            }

            let deleted = delete_playlist(State(state.clone()), Path(uuid))
                .await
                .map_err(|(status, message)| format!("{status}: {message}"))?
                .0;
            if deleted.uuid != uuid
                || state
                    .store
                    .playlists()
                    .await
                    .is_ok_and(|playlists| playlists.contains_key(&uuid))
            {
                return Err("playlist was not deleted".into());
            }

            Ok(())
        }
        .await;

        let cleanup = cleanup_test_content(&state, &prefix).await;
        assert!(cleanup.is_ok(), "test cleanup failed: {cleanup:?}");
        assert!(result.is_ok(), "playlist CRUD failed: {result:?}");
    }
}
