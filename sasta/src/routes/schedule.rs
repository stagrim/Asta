use std::collections::HashSet;

use axum::{
    Json,
    extract::{Path, State},
    response::IntoResponse,
};
use chrono::Local;
use hyper::StatusCode;
use serde::{Deserialize, Serialize};
use tracing::{error, instrument};
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

/// Request payload for creating a new schedule.
#[derive(Debug, Deserialize, ToSchema)]
#[schema(example = json!({
    "name": "Weekday Schedule",
    "playlist": "660e8400-e29b-41d4-a716-446655440000"
}))]
pub struct CreateSchedule {
    /// Unique human-readable name for the schedule. Must not match any existing schedule's name.
    #[schema(example = "Weekday Schedule")]
    pub name: String,
    /// UUID of the default fallback playlist shown when no scheduled time window is active.
    pub playlist: Uuid,
}

/// Representation of a schedule returned by the API.
#[derive(Serialize, ToSchema)]
#[schema(example = json!({
    "uuid": "550e8400-e29b-41d4-a716-446655440000",
    "name": "Weekday Schedule",
    "playlist": "660e8400-e29b-41d4-a716-446655440000",
    "scheduled": [
        {
            "id": "11111111-2222-3333-4444-555555555555",
            "playlist": "99999999-8888-7777-6666-555555555555",
            "start": "0 0 10 * * Mon-Fri *",
            "end": "0 0 14 * * Mon-Fri *"
        }
    ]
}))]
pub struct ReadSchedule {
    /// Unique identifier of the schedule.
    pub uuid: Uuid,
    /// Unique human-readable name of the schedule.
    #[schema(example = "Weekday Schedule")]
    pub name: String,
    /// UUID of the default fallback playlist played when no entry in `scheduled` is currently active.
    pub playlist: Uuid,
    /// Priority-ordered list of time-slotted playlists, or `null` if no time slots are configured.
    /// Earlier entries take priority over later entries if their active windows overlap.
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

/// Real-time evaluation of which playlist a schedule is currently showing and when it will next switch.
#[derive(Serialize, ToSchema)]
#[schema(example = json!({
    "current": "99999999-8888-7777-6666-555555555555",
    "next": {
        "moment": "2026-10-12T14:00:00+02:00",
        "playlist": "660e8400-e29b-41d4-a716-446655440000"
    }
}))]
pub struct ScheduleInfo {
    /// UUID of the playlist that is currently active at the server's local time.
    pub current: Uuid,
    /// Details of the next upcoming playlist switch, or `null` if no future transition will occur.
    pub next: Option<NextMoment>,
}

/// Information about the next scheduled transition point.
#[derive(Serialize, ToSchema, Debug)]
pub struct NextMoment {
    /// RFC3339 formatted date-time string of when the playlist switch will occur.
    #[schema(example = "2026-10-12T14:00:00+02:00")]
    pub moment: String,
    /// UUID of the playlist that will become active at `moment`.
    pub playlist: Uuid,
}

/// Request payload for updating an existing schedule.
#[derive(Debug, Deserialize, ToSchema)]
#[schema(example = json!({
    "name": "Weekday Schedule",
    "playlist": "660e8400-e29b-41d4-a716-446655440000",
    "scheduled": [
        {
            "id": "11111111-2222-3333-4444-555555555555",
            "playlist": "99999999-8888-7777-6666-555555555555",
            "start": "0 0 10 * * Mon-Fri *",
            "end": "0 0 14 * * Mon-Fri *"
        }
    ]
}))]
pub struct UpdateSchedule {
    /// Updated name for the schedule. Must not conflict with any other schedule's name.
    #[schema(example = "Weekday Schedule")]
    pub name: String,
    /// UUID of the default fallback playlist.
    pub playlist: Uuid,
    /// Priority-ordered list of scheduled playlist windows.
    ///
    /// **Constraints:**
    /// - Earlier entries have higher priority than later entries when time windows overlap.
    /// - Every playlist UUID in the schedule (including the fallback `playlist` and all `scheduled` entries) must be unique within this schedule.
    /// - `start` and `end` must be valid 7-field cron expressions (`sec min hour day_of_month month day_of_week year`).
    pub scheduled: Option<Vec<schedule::ScheduledPlaylistInput>>,
}
type Response = Result<Json<ReadSchedule>, (StatusCode, String)>;

#[utoipa::path(
    post,
    path = "/",
    tag = "schedule",
    summary = "Create a new schedule",
    description = "Creates a new schedule with the given `name` and default fallback `playlist` (and an initially empty `scheduled` list). Use `PUT /api/schedule/{uuid}` to configure time-slotted playlists.",
    request_body(content = CreateSchedule),
    responses(
        (status = 200, description = "Schedule created", body = ReadSchedule),
        (status = 400, description = "Bad Request (schedule name is already taken)", body = String,
            example = json!("Avoid using the name Weekday Schedule as it is already used by another Schedule")
        ),
        (status = 500, description = "Server Error", body = String)
    )
)]
#[instrument(level = "debug", skip(state))]
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
        .create_schedule(uuid, &schedule)
        .await
        .map_err(internal)?;
    Ok(Json((uuid, schedule).into()))
}

#[utoipa::path(
    get,
    path = "/",
    tag = "schedule",
    summary = "List all schedules",
    description = "Returns all schedules along with their default fallback playlist and priority-ordered scheduled time slots.",
    responses(
        (status = 200, description = "Get all Schedules", body = inline(Vec<ReadSchedule>),
            example = json!(
                vec![
                    ReadSchedule { uuid: Uuid::new_v4(), name: "name1".into(), playlist: Uuid::new_v4(), scheduled: Some(vec![
                        schedule::ScheduledPlaylistInput {
                            id: Uuid::new_v4(),
                            playlist: Uuid::new_v4(),
                            start: "0 0 10 * * Mon-Fri *".into(),
                            end: "0 0 14 * * Mon-Fri *".into()
                        }
                    ]) },
                    ReadSchedule { uuid: Uuid::new_v4(), name: "name2".into(), playlist: Uuid::new_v4(), scheduled: Some(vec![]) }
                ]
            )
        ),
        (status = 500, description = "Server Error", body = String)
    )
)]
#[instrument(level = "debug", skip(state))]
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
    summary = "Get active playlist and next transition for a schedule",
    description = "Evaluates the schedule against the server's current local time and returns the UUID of the `current` active playlist as well as the `next` upcoming transition (`moment` and target `playlist`), if any.",
    params(
        ("uuid" = Uuid, Path, description = "Uuid of Schedule to fetch info for")
    ),
    responses(
        (status = 200, description = "Schedule information", body = ScheduleInfo),
        (status = 404, description = "Schedule not found")
    )
)]
#[instrument(level = "debug", skip(state))]
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
            moment: time.to_rfc3339(),
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
    summary = "Update a schedule",
    description = "Updates the schedule's name, fallback `playlist`, and priority-ordered `scheduled` time slots.\n\n**Validation rules:**\n- `name` must be unique among all schedules.\n- Every playlist UUID used in the schedule (both the fallback `playlist` and each entry in `scheduled`) must be distinct.\n- Every `start` and `end` expression must be a valid 7-field cron string (`sec min hour day_of_month month day_of_week year`).",
    request_body(content = UpdateSchedule),
    responses(
        (status = 200, description = "Schedule updated", body = ReadSchedule),
        (status = 400, description = "Bad Request (validation failed)", body = String, examples(
            ("not_found" = (
                summary = "No Schedule found with given Uuid",
                value = json!("No Schedule with the Uuid <uuid> was found")
            )),
            ("name_taken" = (
                summary = "Name is already used by another Schedule",
                value = json!("Avoid using the name <name> as it is already used by another Schedule")
            )),
            ("duplicate_playlist" = (
                summary = "Same Playlist used more than once in Schedule",
                value = json!("Must not use the same Playlist more than once in a Schedule")
            )),
            ("invalid_cron" = (
                summary = "Invalid 7-field cron expression",
                value = json!("invalid cron expression '0 0 10 * *'")
            ))
        )),
        (status = 500, description = "Server Error", body = String)
    ),
    params(
        ("uuid" = Uuid, Path, description = "Uuid of Schedule to update")
    )
)]
#[instrument(level = "debug", skip(state))]
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
        .update_schedule(uuid, &schedule)
        .await
        .map_err(internal)?;
    Ok(Json((uuid, schedule).into()))
}

#[utoipa::path(
    delete,
    path = "/{uuid}",
    tag = "schedule",
    summary = "Delete a schedule",
    description = "Deletes the schedule with the specified UUID. Deletion is rejected with `400 Bad Request` if any Display is currently configured to use this schedule.",
    responses(
        (status = 200, description = "Schedule deleted", body = ReadSchedule,
            example = json!(
                ReadSchedule { uuid: Uuid::new_v4(), name: "name".into(), playlist: Uuid::new_v4(), scheduled: None }

            )
        ),
        (status = BAD_REQUEST, description = "Schedule not found or still used by a Display", body = String, examples(
            ("error_1" = (
                summary = "Display(s) depend on Schedule",
                value = json!("Unable to delete Schedule since a Display depends on it")
            )),
            ("error_2" = (
                summary = "No Schedule exists with given Uuid",
                value = json!(
                    format!("No Schedule with the Uuid <uuid> was found")
                )
            ))
        )),
        (status = 500, description = "Server Error", body = String)
    ),
    params(
        ("uuid" = Uuid, Path, description = "Uuid of Schedule to delete")
    )
)]
#[instrument(level = "debug", skip(state))]
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
