use std::collections::HashSet;

use axum::{
    Json,
    extract::{Path, State},
    response::IntoResponse,
};
use chrono::Local;
use hyper::StatusCode;
use serde::{Deserialize, Serialize};
use tracing::error;
use utoipa::ToSchema;
use utoipa_axum::{router::OpenApiRouter, routes};
use uuid::Uuid;

use crate::{
    AppState,
    store::{
        schedule::{self, Moment, Schedule},
        store::DisplayMaterial,
    },
};

#[derive(Deserialize, ToSchema)]
pub struct CreateSchedule {
    pub name: String,
    pub playlist: Uuid,
}
#[derive(Serialize, ToSchema)]
pub struct ReadSchedule {
    pub uuid: Uuid,
    pub name: String,
    pub playlist: Uuid,
    pub scheduled: Option<Vec<schedule::ScheduledPlaylistInput>>,
}
impl From<(Uuid, schedule::Schedule)> for ReadSchedule {
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
#[derive(Serialize, ToSchema)]
pub struct ScheduleInfo {
    pub current: Uuid,
    pub next: Option<NextMoment>,
}
#[derive(Serialize, ToSchema)]
pub struct NextMoment {
    pub in_ms: u64,
    pub playlist: Uuid,
}
#[derive(Deserialize, ToSchema)]
pub struct UpdateSchedule {
    pub name: String,
    pub playlist: Uuid,
    pub scheduled: Option<Vec<schedule::ScheduledPlaylistInput>>,
}
type Response = Result<Json<ReadSchedule>, (StatusCode, String)>;

#[utoipa::path(
    post,
    path = "/",
    tag = "schedule",
    request_body(content = CreateSchedule),
    responses(
        (status = 200, description = "Schedule created", body = CreateSchedule),
        (status = 400, description = "Bad Request", body = CreateSchedule),
        (status = 500, description = "Server Error", body = CreateSchedule)
    )
)]
async fn create_schedule(
    State(state): State<AppState>,
    Json(input): Json<CreateSchedule>,
) -> Response {
    let schedules = state.store.schedules().await.map_err(internal)?;
    if schedules.values().any(|s| s.name == input.name) {
        return Err((
            StatusCode::BAD_REQUEST,
            format!(
                "Avoid using the name {} as it is already used by another Schedule",
                input.name
            ),
        ));
    }
    let uuid = Uuid::new_v4();
    let schedule = Schedule::new(input.name, vec![], input.playlist).map_err(bad_request)?;
    state
        .store
        .create_schedule(uuid, schedule.clone())
        .await
        .map_err(internal)?;
    Ok(Json((uuid, schedule).into()))
}

#[utoipa::path(
    get,
    path = "/",
    tag = "schedule",
    responses(
        (status = 200, description = "Get all Schedules", body = inline(Vec<ReadSchedule>),
            example = json!(
                vec![
                    ReadSchedule { uuid: Uuid::new_v4(), name: "name1".into(), playlist: Uuid::new_v4(), scheduled: Some(vec![
                        schedule::ScheduledPlaylistInput {
                            playlist: Uuid::new_v4(),
                            start: "0 0 10 * * Mon-Fri *".into(),
                            end: "0 0 14 * * Mon-Fri *".into()
                        }
                    ]) },
                    ReadSchedule { uuid: Uuid::new_v4(), name: "name2".into(), playlist: Uuid::new_v4(), scheduled: Some(vec![]) }
                ]
            )
        ),
    )
)]
async fn read_schedules(
    State(state): State<AppState>,
) -> Result<Json<Vec<ReadSchedule>>, (StatusCode, String)> {
    let schedules = state.store.schedules().await.map_err(internal)?;
    Ok(Json(schedules.into_iter().map(Into::into).collect()))
}

#[utoipa::path(
    get,
    path = "/{uuid}",
    tag = "schedule",
    params(
        ("uuid" = Uuid, Path, description = "Uuid of Schedule to fetch info for")
    ),
    responses(
        (status = 200, description = "Schedule information", body = ScheduleInfo),
        (status = 404, description = "Schedule not found")
    )
)]
async fn schedule_info(State(state): State<AppState>, Path(uuid): Path<Uuid>) -> impl IntoResponse {
    let schedule = match state.store.schedule(uuid).await {
        Ok(Some(s)) => s,
        Ok(None) => return Err(format!("Schedule '{uuid}' not found")),
        Err(e) => return Err(e.to_string()),
    };
    let now = Local::now();
    let next = schedule
        .next_schedule(&now)
        .map(|Moment { time, playlist }| NextMoment {
            in_ms: (time - now).num_milliseconds().max(0) as u64,
            playlist,
        });
    Ok(Json(ScheduleInfo {
        current: schedule.current_playlist(&now),
        next,
    }))
}

#[utoipa::path(
    put,
    path = "/{uuid}",
    tag = "schedule",
    request_body(content = UpdateSchedule),
    responses(
        (status = 200, description = "Schedule updated", body = ReadSchedule),
        (status = 400, description = "Bad Request", body = String),
        (status = 500, description = "Server Error", body = String)
    ),
    params(
        ("uuid" = Uuid, Path, description = "Uuid of Schedule to update")
    )
)]
async fn update_schedule(
    State(state): State<AppState>,
    Path(uuid): Path<Uuid>,
    Json(input): Json<UpdateSchedule>,
) -> Response {
    let schedules = state.store.schedules().await.map_err(internal)?;
    if !schedules.contains_key(&uuid) {
        return Err((
            StatusCode::BAD_REQUEST,
            format!("No Schedule with the Uuid {uuid} was found"),
        ));
    }
    if schedules
        .iter()
        .any(|(id, s)| *id != uuid && s.name == input.name)
    {
        return Err((
            StatusCode::BAD_REQUEST,
            format!(
                "Avoid using the name {} as it is already used by another Schedule",
                input.name
            ),
        ));
    }
    if let Some(scheduled) = &input.scheduled {
        let mut ids = HashSet::from([input.playlist]);
        if !scheduled.iter().all(|s| ids.insert(s.playlist)) {
            return Err((
                StatusCode::BAD_REQUEST,
                "Must not use the same Playlist more than once in a Schedule".into(),
            ));
        }
    }
    let schedule = Schedule::new(
        input.name,
        input.scheduled.unwrap_or_default(),
        input.playlist,
    )
    .map_err(bad_request)?;
    state
        .store
        .update_schedule(uuid, schedule.clone())
        .await
        .map_err(internal)?;
    Ok(Json((uuid, schedule).into()))
}

#[utoipa::path(
    delete,
    path = "/{uuid}",
    tag = "schedule",
    responses(
        (status = 200, description = "Schedule deleted", body = ReadSchedule,
            example = json!(
                ReadSchedule { uuid: Uuid::new_v4(), name: "name".into(), playlist: Uuid::new_v4(), scheduled: None }

            )
        ),
        (status = BAD_REQUEST, body = String, examples(
            ("error_1" = (
                summary = "Display(s) depend on Schedule",
                value = json!(
                    format!("Unable to delete Schedule since the Displays (<displays>) depend on it")
                )
            )),
            ("error_2" = (
                summary = "No Schedule exists with given Uuid",
                value = json!(
                    format!("No Schedule with the Uuid <uuid> was found")
                )
            ))
        ))
    ),
    params(
        ("uuid" = Uuid, Path, description = "Uuid of Schedule to delete")
    )
)]
async fn delete_schedule(State(state): State<AppState>, Path(uuid): Path<Uuid>) -> Response {
    let schedule = state
        .store
        .schedule(uuid)
        .await
        .map_err(internal)?
        .ok_or_else(|| {
            (
                StatusCode::BAD_REQUEST,
                format!("No Schedule with the Uuid {uuid} was found"),
            )
        })?;
    let displays = state.store.displays().await.map_err(internal)?;
    if displays
        .values()
        .any(|d| matches!(d.display_material, DisplayMaterial::Schedule(id) if id == uuid))
    {
        return Err((
            StatusCode::BAD_REQUEST,
            "Unable to delete Schedule since a Display depends on it".into(),
        ));
    }
    state.store.delete_schedule(uuid).await.map_err(internal)?;
    Ok(Json((uuid, schedule).into()))
}

fn internal<E: std::fmt::Display>(e: E) -> (StatusCode, String) {
    error!("Redis error: {e}");
    (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
}
fn bad_request(e: String) -> (StatusCode, String) {
    (StatusCode::BAD_REQUEST, e)
}
pub fn schedule_router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(create_schedule))
        .routes(routes!(read_schedules))
        .routes(routes!(schedule_info))
        .routes(routes!(update_schedule))
        .routes(routes!(delete_schedule))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::routes::test_support::{cleanup_test_content, test_prefix};

    #[tokio::test]
    async fn schedule_routes_support_crud() {
        let state = crate::routes::test_support::app_state().await;
        let prefix = test_prefix("schedule");
        let initial_name = format!("{prefix}_initial");
        let updated_name = format!("{prefix}_updated");
        let playlist = Uuid::new_v4();
        let updated_playlist = Uuid::new_v4();

        let result: Result<(), String> = async {
            let created = create_schedule(
                State(state.clone()),
                Json(CreateSchedule {
                    name: initial_name.clone(),
                    playlist,
                }),
            )
            .await
            .map_err(|(status, message)| format!("{status}: {message}"))?
            .0;
            let uuid = created.uuid;
            if created.name != initial_name || created.playlist != playlist {
                return Err("create returned unexpected schedule data".into());
            }

            let listed = read_schedules(State(state.clone()))
                .await
                .map_err(|(status, message)| format!("{status}: {message}"))?
                .0;
            if !listed.iter().any(|schedule| schedule.uuid == uuid) {
                return Err("created schedule was missing from list".into());
            }

            let updated = update_schedule(
                State(state.clone()),
                Path(uuid),
                Json(UpdateSchedule {
                    name: updated_name.clone(),
                    playlist: updated_playlist,
                    scheduled: Some(vec![]),
                }),
            )
            .await
            .map_err(|(status, message)| format!("{status}: {message}"))?
            .0;
            if updated.name != updated_name || updated.playlist != updated_playlist {
                return Err("update returned unexpected schedule data".into());
            }

            let listed = read_schedules(State(state.clone()))
                .await
                .map_err(|(status, message)| format!("{status}: {message}"))?
                .0;
            let persisted = listed
                .iter()
                .find(|schedule| schedule.uuid == uuid)
                .ok_or_else(|| "updated schedule was missing from list".to_string())?;
            if persisted.name != updated_name || persisted.playlist != updated_playlist {
                return Err("updated schedule was not persisted".into());
            }

            let deleted = delete_schedule(State(state.clone()), Path(uuid))
                .await
                .map_err(|(status, message)| format!("{status}: {message}"))?
                .0;
            if deleted.uuid != uuid
                || state
                    .store
                    .schedules()
                    .await
                    .is_ok_and(|schedules| schedules.contains_key(&uuid))
            {
                return Err("schedule was not deleted".into());
            }

            Ok(())
        }
        .await;

        let cleanup = cleanup_test_content(&state, &prefix).await;
        assert!(cleanup.is_ok(), "test cleanup failed: {cleanup:?}");
        assert!(result.is_ok(), "schedule CRUD failed: {result:?}");
    }
}
