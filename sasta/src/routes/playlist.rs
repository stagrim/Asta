use axum::{
    Json,
    extract::{Path, State},
};
use hyper::StatusCode;
use serde::{Deserialize, Serialize};
use tracing::{error, info};
use utoipa::ToSchema;
use utoipa_axum::{router::OpenApiRouter, routes};
use uuid::Uuid;

use crate::{
    AppState,
    store::store::{self, DisplayMaterial},
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

impl From<(Uuid, store::Playlist)> for ReadPlaylist {
    fn from((uuid, p): (Uuid, store::Playlist)) -> Self {
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
    Json(playlist): Json<CreatePlaylist>,
) -> Response {
    info!("[Api] Creating Playlist with name {}", playlist.name);
    let mut store = state.store.lock().await;
    if let Some((uuid, _)) = store
        .content
        .playlists
        .iter()
        .find(|(_, p)| p.name == playlist.name)
    {
        error!("[Api] Name is already used by Playlist {}", uuid);
        return Err((
            StatusCode::BAD_REQUEST,
            format!(
                "Avoid using the name {} as it is already used by another Playlist",
                playlist.name
            ),
        ));
    }

    let uuid = Uuid::new_v4();
    info!("[Api] Generated Uuid {uuid} for new Playlist");

    if let Err(e) = store.create_playlist(uuid, playlist.name).await {
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Could not write changes to db ({e})"),
        ));
    }

    return if let Some(p) = store.content.playlists.get(&uuid) {
        info!("[Api] Created Playlist {uuid}");
        Ok(Json((uuid, p.clone()).into()))
    } else {
        error!("[Api] No Playlist with {uuid} could be found while reading after write");
        Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Something went wrong with the creation"),
        ))
    };
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
async fn read_playlist(State(state): State<AppState>) -> Json<Vec<ReadPlaylist>> {
    return Json(
        state
            .store
            .lock()
            .await
            .content
            .playlists
            .iter()
            .map(|(u, p)| (*u, p.clone()).into())
            .collect(),
    );
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
    Json(playlist): Json<UpdatePlaylist>,
) -> Response {
    info!("[Api] Updating Playlist {uuid}");
    let mut store = state.store.lock().await;
    if !store.content.playlists.contains_key(&uuid) {
        error!("[Api] No Playlist with {uuid} was found");
        return Err((
            StatusCode::BAD_REQUEST,
            format!("No Playlist with the Uuid {uuid} was found"),
        ));
    }
    if let Some((uuid, _)) = store
        .content
        .playlists
        .iter()
        .find(|(u, p)| p.name == playlist.name && **u != uuid)
    {
        error!("[Api] Name is already used by Playlist {}", uuid);
        return Err((
            StatusCode::BAD_REQUEST,
            format!(
                "Avoid using the name {} as it is already used by another Playlist",
                playlist.name
            ),
        ));
    }

    if let Err(e) = store
        .update_playlist(uuid, playlist.name, playlist.items)
        .await
    {
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Could not write changes to db ({e})"),
        ));
    }

    return if let Some(p) = store.content.playlists.get(&uuid) {
        info!("[Api] Updated and read Playlist {uuid}");
        Ok(Json((uuid, p.clone()).into()))
    } else {
        error!("[Api] Could not find Playlist with {uuid} after update");
        Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Could not find Playlist with {uuid} after update"),
        ))
    };
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
pub(crate) async fn delete_playlist(
    State(state): State<AppState>,
    Path(uuid): Path<Uuid>,
) -> Response {
    info!("[Api] Deleting Playlist {uuid}");
    let res;
    let mut store = state.store.lock().await;

    let dependant_schedules = store
        .content
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
            format!(
                "Unable to delete playlist since the Schedules ({}) depend on it",
                dependant_schedules.join(", ")
            ),
        ));
    }

    let dependant_displays = store
        .content
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
            format!(
                "Unable to delete playlist since the Displays ({}) depend on it",
                dependant_displays.join(", ")
            ),
        ));
    }

    if let Some(d) = store.content.playlists.get(&uuid) {
        res = Ok(Json((uuid, d.clone()).into()));
    } else {
        error!("[Api] No Playlist with {uuid} was found");
        return Err((
            StatusCode::BAD_REQUEST,
            format!("No Playlist with the Uuid {uuid} was found"),
        ));
    }

    if let Err(e) = store.delete_playlist(uuid).await {
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Could not write changes to db ({e})"),
        ));
    }

    info!("[Api] Deleted Playlist {uuid}");
    res
}

pub fn playlist_router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(create_playlist))
        .routes(routes!(read_playlist))
        .routes(routes!(update_playlist))
        .routes(routes!(delete_playlist))
}
