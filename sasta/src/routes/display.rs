use axum::{
    Json,
    extract::{Path, State},
};
use hyper::StatusCode;
use serde::{Deserialize, Serialize};
use tracing::{error, info_span};
use utoipa::ToSchema;
use utoipa_axum::{router::OpenApiRouter, routes};
use uuid::Uuid;

use crate::{
    AppState,
    store::store::{Display, DisplayMaterial},
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
impl From<(Uuid, Display)> for ReadDisplay {
    fn from((uuid, d): (Uuid, Display)) -> Self {
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
    let displays = state.store.displays().await.map_err(internal)?;
    if displays.values().any(|d| d.name == disp.name) {
        return Err((
            StatusCode::BAD_REQUEST,
            format!(
                "Avoid using the name {} as it is already used by another display",
                disp.name
            ),
        ));
    }
    let uuid = disp.uuid.unwrap_or_else(Uuid::new_v4);
    if displays.contains_key(&uuid) {
        return Err((
            StatusCode::BAD_REQUEST,
            format!(
                "Avoid using the Uuid {} as it is already used by another display",
                disp.name
            ),
        ));
    }
    let display = Display {
        name: disp.name,
        display_material: disp.display_material,
    };
    state
        .store
        .create_display(uuid, display.clone())
        .await
        .map_err(internal)?;
    Ok(Json((uuid, display).into()))
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
async fn read_displays(
    State(state): State<AppState>,
) -> Result<Json<Vec<ReadDisplay>>, (StatusCode, String)> {
    let displays = state.store.displays().await.map_err(internal)?;
    Ok(Json(displays.into_iter().map(Into::into).collect()))
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
    let displays = state.store.displays().await.map_err(internal)?;
    if !displays.contains_key(&uuid) {
        return Err((
            StatusCode::BAD_REQUEST,
            format!("No Display with the Uuid {uuid} was found"),
        ));
    }
    if displays
        .iter()
        .any(|(id, d)| *id != uuid && d.name == display.name)
    {
        return Err((
            StatusCode::BAD_REQUEST,
            format!(
                "Avoid using the name {} as it is already used by another display",
                display.name
            ),
        ));
    }
    let display = Display {
        name: display.name,
        display_material: display.display_material,
    };
    state
        .store
        .update_display(uuid, display.clone())
        .await
        .map_err(internal)?;
    Ok(Json((uuid, display).into()))
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
    let display = state
        .store
        .display(uuid)
        .await
        .map_err(internal)?
        .ok_or_else(|| {
            (
                StatusCode::BAD_REQUEST,
                format!("No Display with the Uuid {uuid} was found"),
            )
        })?;
    state.store.delete_display(uuid).await.map_err(internal)?;
    Ok(Json((uuid, display).into()))
}

fn internal<E: std::fmt::Display>(e: E) -> (StatusCode, String) {
    error!("Redis error: {e}");
    (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
}
pub fn display_router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(create_display))
        .routes(routes!(read_displays))
        .routes(routes!(update_display))
        .routes(routes!(delete_display))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::routes::test_support::{cleanup_test_content, test_prefix};

    #[tokio::test]
    async fn display_routes_support_crud() {
        let state = crate::routes::test_support::app_state().await;
        let prefix = test_prefix("display");
        let uuid = Uuid::new_v4();
        let initial_name = format!("{prefix}_initial");
        let updated_name = format!("{prefix}_updated");
        let initial_material = DisplayMaterial::Playlist(Uuid::new_v4());

        let result: Result<(), String> = async {
            let created = create_display(
                State(state.clone()),
                Json(CreateDisplay {
                    uuid: Some(uuid),
                    name: initial_name.clone(),
                    display_material: initial_material,
                }),
            )
            .await
            .map_err(|(status, message)| format!("{status}: {message}"))?
            .0;
            if created.uuid != uuid || created.name != initial_name {
                return Err("create returned unexpected display data".into());
            }

            let listed = read_displays(State(state.clone()))
                .await
                .map_err(|(status, message)| format!("{status}: {message}"))?
                .0;
            if !listed.iter().any(|display| display.uuid == uuid) {
                return Err("created display was missing from list".into());
            }

            let updated = update_display(
                State(state.clone()),
                Path(uuid),
                Json(UpdateDisplay {
                    name: updated_name.clone(),
                    display_material: DisplayMaterial::Schedule(Uuid::new_v4()),
                }),
            )
            .await
            .map_err(|(status, message)| format!("{status}: {message}"))?
            .0;
            if updated.name != updated_name {
                return Err("update returned unexpected display data".into());
            }

            let listed = read_displays(State(state.clone()))
                .await
                .map_err(|(status, message)| format!("{status}: {message}"))?
                .0;
            let persisted = listed
                .iter()
                .find(|display| display.uuid == uuid)
                .ok_or_else(|| "updated display was missing from list".to_string())?;
            if persisted.name != updated_name
                || !matches!(&persisted.display_material, DisplayMaterial::Schedule(_))
            {
                return Err("updated display was not persisted".into());
            }

            let deleted = delete_display(State(state.clone()), Path(uuid))
                .await
                .map_err(|(status, message)| format!("{status}: {message}"))?
                .0;

            if deleted.uuid != uuid
                || state
                    .store
                    .displays()
                    .await
                    .is_ok_and(|displays| displays.contains_key(&uuid))
            {
                return Err("display was not deleted".into());
            }

            Ok(())
        }
        .await;

        let cleanup = cleanup_test_content(&state, &prefix).await;
        assert!(cleanup.is_ok(), "test cleanup failed: {cleanup:?}");
        assert!(result.is_ok(), "display CRUD failed: {result:?}");
    }
}
