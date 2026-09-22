use std::collections::HashSet;

use axum::{
    Json,
    extract::{Path, State},
    response::IntoResponse,
};
use chrono::Local;
use hyper::StatusCode;
use serde::{Deserialize, Serialize};
use tracing::{error, info};
use utoipa::ToSchema;
use utoipa_axum::{router::OpenApiRouter, routes};
use uuid::Uuid;

use crate::{
    AppState,
    store::{
        schedule::{self, Moment},
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
    /// Amount of milliseconds until change
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
    Json(schedule): Json<CreateSchedule>,
) -> Response {
    info!("[Api] Creating Schedule with name {}", schedule.name);
    let mut store = state.store.lock().await;
    if let Some((uuid, _)) = store
        .content
        .schedules
        .iter()
        .find(|(_, s)| s.name == schedule.name)
    {
        error!("[Api] Name is already used by Schedule {}", uuid);
        return Err((
            StatusCode::BAD_REQUEST,
            format!(
                "Avoid using the name {} as it is already used by another Schedule",
                schedule.name
            ),
        ));
    }

    let uuid = Uuid::new_v4();
    info!("[Api] Generated Uuid {uuid} for new Schedule");

    if let Err(e) = store
        .create_schedule(uuid, schedule.name, schedule.playlist)
        .await
    {
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Could not write changes to db ({e})"),
        ));
    }

    return if let Some(s) = store.content.schedules.get(&uuid) {
        info!("[Api] Created Schedule {uuid}");
        Ok(Json((uuid, s.clone()).into()))
    } else {
        error!("[Api] No Schedule with {uuid} could be found while reading after write");
        Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Something went wrong with the creation"),
        ))
    };
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
async fn read_schedules(State(state): State<AppState>) -> Json<Vec<ReadSchedule>> {
    return Json(
        state
            .store
            .lock()
            .await
            .content
            .schedules
            .iter()
            .map(|(u, s)| (*u, s.clone()).into())
            .collect(),
    );
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
    let current_moment = Local::now();
    let store = state.store.lock().await;

    if let Some(schedule) = store.content.schedules.get(&uuid) {
        let next_moment =
            schedule
                .next_schedule(&current_moment)
                .and_then(|Moment { time, playlist }| {
                    Some(NextMoment {
                        in_ms: (time - current_moment).num_milliseconds() as u64,
                        playlist,
                    })
                });

        Ok(Json(ScheduleInfo {
            current: schedule.current_playlist(&current_moment),
            next: next_moment,
        }))
    } else {
        Err(format!("Schedule '{uuid}' not found"))
    }
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
    Json(schedule): Json<UpdateSchedule>,
) -> Response {
    info!("[Api] Updating Schedule {uuid}");
    let mut store = state.store.lock().await;
    if !store.content.schedules.contains_key(&uuid) {
        error!("[Api] No Schedule with {uuid} was found");
        return Err((
            StatusCode::BAD_REQUEST,
            format!("No Schedule with the Uuid {uuid} was found"),
        ));
    }
    if let Some((uuid, _)) = store
        .content
        .schedules
        .iter()
        .find(|(u, s)| s.name == schedule.name && **u != uuid)
    {
        error!("[Api] Name is already used by Schedule {}", uuid);
        return Err((
            StatusCode::BAD_REQUEST,
            format!(
                "Avoid using the name {} as it is already used by another Schedule",
                schedule.name
            ),
        ));
    }

    if let Some(scheduled) = &schedule.scheduled {
        let mut uniq = HashSet::new();
        uniq.insert(schedule.playlist);
        // Checks if any playlist Uuid is a duplicate
        if !scheduled.iter().all(|s| uniq.insert(s.playlist)) {
            error!("[Api] Schedule contains duplicate Playlists");
            return Err((
                StatusCode::BAD_REQUEST,
                format!(
                    "Must not use the same Playlist more than once in a Schedule to avoid server meltdown"
                ),
            ));
        }
    }

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
        return Err((StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into());
    }

    return if let Some(s) = store.content.schedules.get(&uuid) {
        info!("[Api] Updated and read Schedule {uuid}");
        Ok(Json((uuid, s.clone()).into()))
    } else {
        error!("[Api] Could not find Schedule with {uuid} after update");
        Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Could not find Schedule with {uuid} after update to avoid server meltdown"),
        ))
    };
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
    info!("[Api] Deleting Schedule {uuid}");
    let res;
    let mut store = state.store.lock().await;

    let dependant_displays = store
        .content
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
            format!(
                "Unable to delete playlist since the Displays ({}) depend on it",
                dependant_displays.join(", ")
            ),
        ));
    }

    if let Some(s) = store.content.schedules.get(&uuid) {
        res = Ok(Json((uuid, s.clone()).into()));
    } else {
        error!("[Api] No Schedule with {uuid} was found");
        return Err((
            StatusCode::BAD_REQUEST,
            format!("No Schedule with the Uuid {uuid} was found"),
        ));
    }

    if let Err(e) = store.delete_schedule(uuid).await {
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Could not write changes to db ({e})"),
        ));
    }

    info!("[Api] Deleted Schedule {uuid}");
    res
}

pub fn schedule_router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(create_schedule))
        .routes(routes!(read_schedules))
        .routes(routes!(schedule_info))
        .routes(routes!(update_schedule))
        .routes(routes!(delete_schedule))
}
