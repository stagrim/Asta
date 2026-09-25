use axum::{
    Json,
    extract::{Path, State},
};
use hyper::StatusCode;
use serde::{Deserialize, Serialize};
use tracing::error;
use utoipa::ToSchema;
use utoipa_axum::{router::OpenApiRouter, routes};
use uuid::Uuid;

use crate::{
    AppState,
    store::store::{self, Playlist},
};

#[derive(Deserialize, ToSchema)]
pub struct CreatePlaylist {
    pub name: String,
}
#[derive(Serialize, ToSchema)]
pub struct ReadPlaylist {
    pub uuid: Uuid,
    pub name: String,
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
#[derive(Deserialize, ToSchema)]
pub struct UpdatePlaylist {
    pub name: String,
    pub items: Vec<store::PlaylistItem>,
}
type Response = Result<Json<ReadPlaylist>, (StatusCode, String)>;

#[utoipa::path(
    post,
    path = "/",
    tag = "playlist",
    request_body(content = CreatePlaylist),
    responses(
        (status = 200, description = "Playlist created", body = ReadPlaylist),
        (status = 400, description = "Bad Request", body = String),
        (status = 500, description = "Server Error", body = String)
    )
)]
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
        .create_playlist(uuid, playlist.clone())
        .await
        .map_err(internal)?;
    Ok(Json((uuid, playlist).into()))
}

#[utoipa::path(
    get,
    path = "/",
    tag = "playlist",
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
    )
)]
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
    request_body(content = UpdatePlaylist),
    responses(
        (status = 200, description = "Playlist updated", body = ReadPlaylist),
        (status = 400, description = "Bad Request", body = String),
        (status = 500, description = "Server Error", body = String)
    ),
    params(
        ("uuid" = Uuid, Path, description = "Uuid of Playlist to update")
    )
)]
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
        .update_playlist(uuid, playlist.clone())
        .await
        .map_err(internal)?;
    Ok(Json((uuid, playlist).into()))
}

#[utoipa::path(
    delete,
    path = "/{uuid}",
    tag = "playlist",
    responses(
        (status = 200, description = "Playlist deleted", body = ReadPlaylist,
            example = json!(
                ReadPlaylist { uuid: Uuid::new_v4(), name: "name".into(), items: vec![] }
            )
        ),
        (status = BAD_REQUEST, body = String, examples(
            ("error_1" = (
                summary = "Playlist(s) depend on Playlist",
                value = json!(
                    format!("Unable to delete playlist since the Schedules (<schedules>) depend on it")
                )
            )),
            ("error_2" = (
                summary = "No playlist exists with given Uuid",
                value = json!(
                    format!("No Playlist with the Uuid <uuid> was found")
                )
            ))
        ))
    ),
    params(
        ("uuid" = Uuid, Path, description = "Uuid of Playlist to delete")
    )
)]
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
