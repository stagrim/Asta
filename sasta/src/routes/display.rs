use axum::{
    Json,
    extract::{Path, State},
};
use hyper::StatusCode;
use serde::{Deserialize, Serialize};
use tracing::{error, info, info_span};
use utoipa::ToSchema;
use utoipa_axum::{router::OpenApiRouter, routes};
use uuid::Uuid;

use crate::{
    AppState,
    store::store::{self, DisplayMaterial},
};

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateDisplay {
    pub uuid: Option<Uuid>,
    pub name: String,
    pub display_material: DisplayMaterial,
}

#[derive(Serialize, ToSchema)]
pub struct ReadDisplay {
    pub uuid: Uuid,
    pub name: String,
    pub display_material: DisplayMaterial,
}

impl From<(Uuid, store::Display)> for ReadDisplay {
    fn from((uuid, d): (Uuid, store::Display)) -> Self {
        Self {
            uuid,
            name: d.name,
            display_material: d.display_material,
        }
    }
}

#[derive(Deserialize, ToSchema)]
#[schema(title = "UpdateDisplay")]
pub struct UpdateDisplay {
    pub name: String,
    pub display_material: DisplayMaterial,
}

type Response = Result<Json<ReadDisplay>, (StatusCode, String)>;

#[utoipa::path(
    post,
    path = "/",
    tag = "display",
    request_body = CreateDisplay,
    responses(
        (status = 200, description = "Display created", body = ReadDisplay),
        (status = 400, description = "Bad Request (e.g., name taken)", body = String),
        (status = 500, description = "Server Error", body = String)
    )
)]
async fn create_display(
    State(state): State<AppState>,
    Json(disp): Json<CreateDisplay>,
) -> Response {
    info_span!("[Api] Creating Display", display = ?disp);
    let mut store = state.store.lock().await;
    if let Some((uuid, _)) = store
        .content
        .displays
        .iter()
        .find(|(_, d)| d.name == disp.name)
    {
        error!("[Api] Name is already used by Display {}", uuid);
        return Err((
            StatusCode::BAD_REQUEST,
            format!(
                "Avoid using the name {} as it is already used by another display",
                disp.name
            ),
        ));
    }

    if let Some(uuid) = disp.uuid {
        if store.content.displays.contains_key(&uuid) {
            error!("[Api] Uuid is already used by another Display");
            return Err((
                StatusCode::BAD_REQUEST,
                format!(
                    "Avoid using the Uuid {} as it is already used by another display",
                    disp.name
                ),
            ));
        }
    }

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
            format!("Could not write changes to db ({e})"),
        ));
    }

    return if let Some(d) = store.content.displays.get(&uuid).cloned() {
        info!("[Api] Created Display {uuid}");
        Ok(Json((uuid, d).into()))
    } else {
        error!(
            "[Api] No Display with {} could be found while reading after write",
            uuid
        );
        Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Something went wrong with the creation"),
        ))
    };
}

#[utoipa::path(
    get,
    path = "/",
    tag = "display",
    responses(
        (status = 200, description = "Get all Displays", body = inline(Vec<ReadDisplay>),
            example = json!(
                vec![
                    ReadDisplay { uuid: Uuid::new_v4(), name: "name1".into(), display_material: DisplayMaterial::Schedule(Uuid::new_v4()) },
                    ReadDisplay { uuid: Uuid::new_v4(), name: "name2".into(), display_material: DisplayMaterial::Playlist(Uuid::new_v4()) },
                    ReadDisplay { uuid: Uuid::new_v4(), name: "name3".into(), display_material: DisplayMaterial::Schedule(Uuid::new_v4()) }
                ]
            )
        ),
    )
)]
async fn read_displays(State(state): State<AppState>) -> Json<Vec<ReadDisplay>> {
    return Json(
        state
            .store
            .lock()
            .await
            .content
            .displays
            .iter()
            .map(|(u, d)| (*u, d.clone()).into())
            .collect(),
    );
}

#[utoipa::path(
    put,
    path = "/{uuid}",
    tag = "display",
    request_body(content = UpdateDisplay),
    responses(
        (status = 200, description = "Display updated", body = ReadDisplay,
            example = json!(
                ReadDisplay { uuid: Uuid::new_v4(), name: "name".into(), display_material: DisplayMaterial::Schedule(Uuid::new_v4()) }
            )
        ),
        (status = BAD_REQUEST, body = String, examples(
            ("error_1" = (
                summary = "No Display found with given Uuid",
                value = json!(format!("No Display with the Uuid <uuid> was found"))
            )),
            ("error_2" = (
                summary = "Name is already used by another Display",
                value = json!(format!("Avoid using the name <name> as it is already used by another display"))
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
    Json(display): Json<UpdateDisplay>,
) -> Response {
    info!("[Api] Updating Display {uuid}");
    let mut store = state.store.lock().await;
    if !store.content.displays.contains_key(&uuid) {
        error!("[Api] No display with {uuid} was found");
        return Err((
            StatusCode::BAD_REQUEST,
            format!("No Display with the Uuid {uuid} was found"),
        ));
    }
    if let Some((uuid, _)) = store
        .content
        .displays
        .iter()
        .find(|(u, d)| d.name == display.name && **u != uuid)
    {
        error!("[Api] Name is already used by Display {}", uuid);
        return Err((
            StatusCode::BAD_REQUEST,
            format!(
                "Avoid using the name {} as it is already used by another display",
                display.name
            ),
        ));
    }

    if let Err(e) = store
        .update_display(uuid, display.name, display.display_material)
        .await
    {
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Could not write changes to db ({e})"),
        ));
    }

    return if let Some(d) = store.content.displays.get(&uuid) {
        info!("[Api] Updated and read Display {uuid}");
        Ok(Json((uuid, d.clone()).into()))
    } else {
        error!("[Api] Could not find Display with {uuid} after update");
        Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Could not find Display with {uuid} after update"),
        ))
    };
}

#[utoipa::path(
    delete,
    path = "/{uuid}",
    tag = "display",
    responses(
        (status = 200, description = "Display deleted", body = ReadDisplay,
            example = json!(
                ReadDisplay { uuid: Uuid::new_v4(), name: "name".into(), display_material: DisplayMaterial::Schedule(Uuid::new_v4()) }
            )
        ),
        (status = BAD_REQUEST, body = String,
            description = "No Display exists with given Uuid",
            example = json!(
                format!("No Display with the Uuid <uuid> was found")
            )
        )
    ),
    params(
        ("uuid" = Uuid, Path, description = "Uuid of Display to delete")
    )
)]
async fn delete_display(State(state): State<AppState>, Path(uuid): Path<Uuid>) -> Response {
    info!("[Api] Deleting Display {uuid}");
    let mut store = state.store.lock().await;
    let res;
    if let Some(d) = store.content.displays.get(&uuid) {
        res = Ok(Json((uuid, d.clone()).into()));
    } else {
        error!("[Api] No display with {uuid} was found");
        return Err((
            StatusCode::BAD_REQUEST,
            format!("No Display with the Uuid {uuid} was found"),
        ));
    }

    if let Err(e) = store.delete_display(uuid).await {
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Could not write changes to db ({e})"),
        ));
    }

    info!("[Api] Deleted Display {uuid}");
    res
}

pub fn display_router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(create_display))
        .routes(routes!(read_displays))
        .routes(routes!(update_display))
        .routes(routes!(delete_display))
}
