use std::{collections::HashSet, sync::Arc};

use chrono::Local;
use tokio::{
    sync::{Mutex, oneshot},
    time::{Instant, sleep_until},
};
use tracing::{error, info, trace};
use uuid::Uuid;

use crate::store::store::{Change, Store};

use super::schedule::{Moment, Schedule};

/// Starts scheduling loop updating the active playlists when necessary
///
/// Cancels sent token when state has been updated to the active scheduled playlists
pub async fn schedule_loop(store: Arc<Mutex<Store>>, tx: oneshot::Sender<()>) {
    let mut current_moment = Local::now();
    let mut receiver = store.lock().await.receiver();

    let schedules: Vec<(Uuid, Schedule)> = store
        .lock()
        .await
        .content
        .schedules
        .iter()
        .map(|(schedule_uuid, schedule)| (schedule_uuid.clone(), schedule.clone()))
        .collect();

    // Updates all schedules to their current active scheduled playlist
    if !schedules.is_empty() {
        // Does not care about redis error, since only changes internal state.
        // TODO: Make new method which only changes internal state and does not write
        // to db to make this clearer?
        let _ = store
            .lock()
            .await
            .write(|c| {
                schedules.iter().for_each(|(uuid, schedule)| {
                    c.schedules
                        .entry(*uuid)
                        .and_modify(|s| s.playlist = schedule.current_playlist(&current_moment));
                });
                // Change notice not needed since main thread waits on oneshot notice before continuing
                None
            })
            .await;
        info!("[Scheduler] Updated Schedules to current active playlist");
    }

    // Notify oneshot channel that schedules have been updated to active playlists
    if let Err(_) = tx.send(()) {
        error!("[Scheduler] Could not notify listener, sender dropped");
    }

    'main: loop {
        let instant = Instant::now();
        let schedules: Vec<(Uuid, Schedule)> = store
            .lock()
            .await
            .content
            .schedules
            .iter()
            .map(|(schedule_uuid, schedule)| (schedule_uuid.clone(), schedule.clone()))
            .collect();

        let mut moments: Vec<(Uuid, Moment)> = schedules
            .iter()
            .filter_map(
                |(schedule_uuid, schedule)| match schedule.next_schedule(&current_moment) {
                    Some(m) => Some((schedule_uuid.clone(), m)),
                    None => None,
                },
            )
            .collect();

        if moments.is_empty() {
            info!(
                "[Scheduler] No loaded Schedule has any scheduled playlists, waiting on an update to a Schedule..."
            );
            loop {
                match receiver.recv().await {
                    Ok(Change::ScheduleInput(uuids)) => {
                        let read = &store.lock().await.content;
                        if uuids.iter().any(|u| {
                            read.schedules
                                .get(u)
                                .is_some_and(|s| s.has_scheduled_playlists())
                        }) {
                            info!(
                                "[Scheduler] An updated Schedule has scheduled playlists, rerunning loop"
                            );
                            break;
                        }
                    }
                    Err(e) => error!("[Scheduler] RecvError: {e}"),
                    _ => info!("[Scheduler] Non relevant change received, continue waiting"),
                }
            }
            continue;
        }

        let closest_time = moments.iter().min_by_key(|(_, m)| m.time).unwrap().1.time;

        moments = moments
            .into_iter()
            .filter(|(_, m)| m.time == closest_time)
            .collect();

        let sleep = match (closest_time - Local::now()).to_std() {
            Ok(d) => instant + d,
            Err(_) => instant,
        };

        info!(
            "[Scheduler] Sleeping for {:?} until {} to change active playlists",
            sleep.duration_since(instant),
            closest_time.to_string()
        );

        loop {
            tokio::select! {
                _ = sleep_until(sleep) => {
                    info!("[Scheduler] Breaking");
                    break
                },
                change = receiver.recv() => {
                    match change {
                        //TODO: When updating a schedule, the new schedule is overridden in the API, and since the 'set current block' lies before the loop, they are never reverted to the present version
                        Ok(Change::ScheduleInput(uuids)) => {
                            info!("[Scheduler] Schedules updated, rerunning loop");
                            let _ = store.lock().await.write(|c| {
                                uuids.iter().for_each(|uuid| {
                                    c.schedules
                                        .entry(*uuid)
                                        .and_modify(|s| s.playlist = s.current_playlist(&current_moment));
                                });
                                Some(Change::Schedule(uuids))
                            }).await;
                            continue 'main
                        },
                        Err(e) => error!("[Scheduler] RecvError: {e}"),
                        _ => trace!("[Scheduler] Non relevant change received, continue waiting"),
                    }
                },
            }
        }
        info!("[Scheduler] Sleep done, updating active playlists");

        let schedule_active_playlist_pairs = moments
            .iter()
            .map(|(u, m)| (*u, m.playlist))
            .inspect(|(uuid, _)| info!("[Scheduler] Updating Schedule {uuid} active playlist"))
            .collect::<Vec<_>>();

        // Updates all Schedules' Playlists to its current active playlist
        let _ = store
            .lock()
            .await
            .write(|content| {
                schedule_active_playlist_pairs
                    .iter()
                    .for_each(|(schedule, playlist)| {
                        content
                            .schedules
                            .entry(*schedule)
                            .and_modify(|s| s.playlist = *playlist);
                    });
                Some(Change::Schedule(HashSet::from_iter(
                    schedule_active_playlist_pairs.iter().map(|v| v.0),
                )))
            })
            .await;
        current_moment = closest_time;
    }
}
