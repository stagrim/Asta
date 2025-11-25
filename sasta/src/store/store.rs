use std::collections::{HashMap, HashSet};

use chrono::Local;
use redis::{aio::ConnectionManager, Client, JsonAsyncCommands, RedisError};
use serde::{Deserialize, Deserializer, Serialize};
use tokio::{
    sync::{
        broadcast::{self, Receiver, Sender},
        oneshot, Mutex, RwLock, RwLockReadGuard, RwLockWriteGuard,
    },
    time::{sleep_until, Instant},
};
use tracing::{error, error_span, info, trace, warn, warn_span};
use ts_rs::TS;
use utoipa::ToSchema;
use uuid::Uuid;

use super::schedule::{self, Moment, Schedule};

#[derive(Serialize, Debug, Clone)]
pub struct Display {
    pub name: String,
    pub display_material: DisplayMaterial,
}

// Explicit implementation to cover for the new format to cover backward compatibility
impl<'de> Deserialize<'de> for Display {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        // 10 min of my life I will never get back...
        #[derive(Deserialize)]
        pub struct NewDisplay {
            pub name: String,
            pub display_material: DisplayMaterial,
        }

        #[derive(Deserialize)]
        struct OldDisplay {
            name: String,
            schedule: Uuid,
        }

        #[derive(Deserialize)]
        #[serde(untagged)]
        enum TempDisplay {
            New(NewDisplay),
            Old(OldDisplay),
        }

        match TempDisplay::deserialize(deserializer)? {
            TempDisplay::New(NewDisplay {
                name,
                display_material,
            }) => Ok(Display {
                name,
                display_material,
            }),
            TempDisplay::Old(OldDisplay { name, schedule }) => Ok(Display {
                name,
                display_material: DisplayMaterial::Schedule(schedule),
            }),
        }
    }
}

// TODO: make this the rust way instead, and do some fallback magic in the Deserialize process instead from the db... Probably much better.
#[derive(Deserialize, Serialize, Debug, ToSchema, TS, Clone)]
#[ts(export, export_to = "api_bindings/create/")]
#[serde(rename_all = "snake_case")]
#[serde(tag = "type", content = "uuid")]
pub enum DisplayMaterial {
    Schedule(#[ts(type = "string")] Uuid),
    Playlist(#[ts(type = "string")] Uuid),
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Playlist {
    pub name: String,
    pub items: Vec<PlaylistItem>,
}

#[derive(Deserialize, Serialize, Debug, Clone, ToSchema, TS)]
#[serde(tag = "type")]
#[ts(export, export_to = "api_bindings/update/")]
pub enum PlaylistItem {
    #[serde(rename = "WEBSITE")]
    Website {
        #[serde(alias = "name")]
        id: String,
        settings: WebsiteData,
    },
    #[serde(rename = "TEXT")]
    Text {
        #[serde(alias = "name")]
        id: String,
        settings: TextData,
    },
    #[serde(rename = "IMAGE")]
    Image {
        #[serde(alias = "name")]
        id: String,
        settings: ImageData,
    },
    #[serde(rename = "BACKGROUND_AUDIO")]
    BackgroundAudio {
        #[serde(alias = "name")]
        id: String,
        settings: ImageData,
    },
}

#[derive(Deserialize, Serialize, Debug, Clone, ToSchema, TS)]
#[ts(export, export_to = "api_bindings/update/")]
pub struct WebsiteData {
    pub url: String,
    pub duration: u64,
}

#[derive(Deserialize, Serialize, Debug, Clone, ToSchema, TS)]
#[ts(export, export_to = "api_bindings/update/")]
pub struct TextData {
    pub text: String,
    pub duration: u64,
}

#[derive(Deserialize, Serialize, Debug, Clone, ToSchema, TS)]
#[ts(export, export_to = "api_bindings/update/")]
pub struct ImageData {
    pub src: String,
    pub duration: u64,
}

// TODO: Replace Content, and use redis as only storage
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Content {
    pub displays: HashMap<Uuid, Display>,
    pub playlists: HashMap<Uuid, Playlist>,
    pub schedules: HashMap<Uuid, Schedule>,
}

pub struct Store {
    con: Mutex<ConnectionManager>,
    sender: Sender<Change>,
    content: RwLock<Content>,
}

impl Store {
    pub async fn new(redis_url: &str) -> Self {
        let client = Client::open(redis_url).unwrap();
        let mut con = ConnectionManager::new(client).await.unwrap();
        let (sender, _) = broadcast::channel(5);
        let content = RwLock::new(Self::read_file(&mut con).await);

        let s = Store {
            con: Mutex::new(con),
            sender,
            content,
        };
        s
    }

    async fn read_file(con: &mut ConnectionManager) -> Content {
        match con.json_get::<_, _, String>("content", ".").await {
            Ok(str) => {
                // println!("{}", str);
                serde_json::from_str(&str).unwrap()
            }
            Err(e) => {
                warn_span!("[Store] could not parse file content, starting with a blank state", redis = ?e);
                Content {
                    displays: HashMap::new(),
                    playlists: HashMap::new(),
                    schedules: HashMap::new(),
                }
            }
        }
    }

    /// Updates all Schedules' Playlists to the (schedule_uuid, active_playlist_uuid) pairs
    async fn update_schedule_active_playlist(
        &self,
        vec: Vec<(Uuid, Uuid)>,
    ) -> Result<(), RedisError> {
        // Exit if schedule does not exists or if it is already set to the given playlist
        // if !self.read().await.schedules.contains_key(&schedule) || self.read().await.schedules.get(&schedule).unwrap().playlist == active_playlist {
        //     return;
        // }

        self.write(|mut c| {
            vec.iter().for_each(|(schedule, playlist)| {
                c.schedules
                    .entry(*schedule)
                    .and_modify(|s| s.playlist = *playlist);
            });
            Some(Change::Schedule(HashSet::from_iter(
                vec.iter().map(|v| v.0),
            )))
        })
        .await
    }

    /// Starts scheduling loop updating the active playlists when necessary
    ///
    /// Cancels sent token when state has been updated to the active scheduled playlists
    pub async fn schedule_loop(&self, tx: oneshot::Sender<()>) {
        let mut current_moment = Local::now();
        let mut receiver = self.receiver();

        let schedules: Vec<(Uuid, Schedule)> = self
            .read()
            .await
            .schedules
            .iter()
            .map(|(schedule_uuid, schedule)| (schedule_uuid.clone(), schedule.clone()))
            .collect();

        // Updates all schedules to their current active scheduled playlist
        if !schedules.is_empty() {
            // Does not care about redis error, since only changes internal state.
            // TODO: Make new method which only changes internal state and does not write
            // to db to make this clearer?
            let _ = self
                .write(|mut c| {
                    schedules.iter().for_each(|(uuid, schedule)| {
                        c.schedules.entry(*uuid).and_modify(|s| {
                            s.playlist = schedule.current_playlist(&current_moment)
                        });
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
            let schedules: Vec<(Uuid, Schedule)> = self
                .read()
                .await
                .schedules
                .iter()
                .map(|(schedule_uuid, schedule)| (schedule_uuid.clone(), schedule.clone()))
                .collect();

            let mut moments: Vec<(Uuid, Moment)> = schedules
                .iter()
                .filter_map(|(schedule_uuid, schedule)| {
                    match schedule.next_schedule(&current_moment) {
                        Some(m) => Some((schedule_uuid.clone(), m)),
                        None => None,
                    }
                })
                .collect();

            if moments.is_empty() {
                info!("[Scheduler] No loaded Schedule has any scheduled playlists, waiting on an update to a Schedule...");
                loop {
                    match receiver.recv().await {
                        Ok(Change::ScheduleInput(uuids)) => {
                            let read = self.read().await;
                            if uuids.iter().any(|u| {
                                read.schedules
                                    .get(u)
                                    .is_some_and(|s| s.has_scheduled_playlists())
                            }) {
                                info!("[Scheduler] An updated Schedule has scheduled playlists, rerunning loop");
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
                                let _ = self.write(|mut c| {
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

            let _ = self
                .update_schedule_active_playlist(
                    moments
                        .iter()
                        .map(|(u, m)| (*u, m.playlist))
                        .inspect(|(uuid, _)| {
                            info!("[Scheduler] Updating Schedule {uuid} active playlist")
                        })
                        .collect::<Vec<_>>(),
                )
                .await;
            current_moment = closest_time;
        }
    }

    /// Returns receiver handle to a watch channel which gets notified if store has been updated
    pub fn receiver(&self) -> Receiver<Change> {
        self.sender.subscribe()
    }

    pub async fn read(&self) -> RwLockReadGuard<Content> {
        self.content.read().await
    }

    /// Runs closure with lock write guard handle given as argument
    /// and sends a message signalling a state change once it is done
    async fn write<F>(&self, fun: F) -> Result<(), RedisError>
    where
        F: FnOnce(RwLockWriteGuard<Content>) -> Option<Change>,
    {
        let c = self.content.write().await;
        let changes = fun(c);
        info!("[Store] Sending changes after write: {changes:?}");
        if let Some(c) = changes {
            if let Err(e) = self.sender.send(c) {
                warn!("[Store] No active channels to listen in ({})", e)
            }
        }
        info!("[Store] writing new state to db");
        let mut con = self.con.lock().await;
        let content = &self.content.read().await.clone();
        if let Err(error) = con
            .json_set::<_, _, _, String>("content", "$", &content)
            .await
        {
            error_span!("Redis Error", ?error);
            Err(error)
        } else {
            Ok(())
        }
    }

    /// Creates a new display
    ///
    /// Overrides existing display with same uuid
    pub async fn create_display(
        &self,
        uuid: Uuid,
        name: String,
        display_material: DisplayMaterial,
    ) -> Result<(), RedisError> {
        self.write(|mut c| {
            c.displays.insert(
                uuid,
                Display {
                    name,
                    display_material,
                },
            );
            Some(Change::Display(HashSet::from([uuid])))
        })
        .await
    }

    /// Creates a new playlist
    ///
    /// Overrides existing playlist with same uuid
    pub async fn create_playlist(&self, uuid: Uuid, name: String) -> Result<(), RedisError> {
        self.write(|mut c| {
            c.playlists.insert(
                uuid,
                Playlist {
                    name,
                    items: vec![],
                },
            );
            Some(Change::Playlist(HashSet::from([uuid])))
        })
        .await
    }

    /// Creates a new Schedule
    ///
    /// Overrides existing Schedule with same uuid
    pub async fn create_schedule(
        &self,
        uuid: Uuid,
        name: String,
        playlist: Uuid,
    ) -> Result<(), RedisError> {
        self.write(|mut c| {
            c.schedules
                .insert(uuid, Schedule::new(name, vec![], playlist).unwrap());
            Some(Change::Schedule(HashSet::from([uuid])))
        })
        .await
    }

    /// Updates the display with the given Uuid
    ///
    /// Does nothing if no such display is found
    pub async fn update_display(
        &self,
        uuid: Uuid,
        name: String,
        display_material: DisplayMaterial,
    ) -> Result<(), RedisError> {
        self.write(|mut c| {
            c.displays.entry(uuid).and_modify(|d| {
                *d = Display {
                    name,
                    display_material,
                }
            });
            Some(Change::Display(HashSet::from([uuid])))
        })
        .await
    }

    /// Updates the playlist with the given Uuid
    ///
    /// Does nothing if no such playlist is found
    pub async fn update_playlist(
        &self,
        uuid: Uuid,
        name: String,
        items: Vec<PlaylistItem>,
    ) -> Result<(), RedisError> {
        self.write(|mut c| {
            c.playlists
                .entry(uuid)
                .and_modify(|p| *p = Playlist { name, items });
            Some(Change::Playlist(HashSet::from([uuid])))
        })
        .await
    }

    /// Updates the Schedule with the given Uuid
    ///
    /// Does nothing if no such Schedule is found
    pub async fn update_schedule(
        &self,
        uuid: Uuid,
        name: String,
        playlist: Uuid,
        schedules: Vec<schedule::ScheduledPlaylistInput>,
    ) -> Result<(), String> {
        let schedule = match Schedule::new(name, schedules, playlist) {
            Ok(s) => s,
            Err(e) => return Err(e),
        };
        self.write(|mut c| {
            c.schedules.entry(uuid).and_modify(|s| *s = schedule);
            Some(Change::ScheduleInput(HashSet::from([uuid])))
        })
        .await
        .map_err(|e| e.to_string())
    }

    /// Deletes the display with the given Uuid
    ///
    /// Does nothing if no such display is found
    pub async fn delete_display(&self, uuid: Uuid) -> Result<(), RedisError> {
        self.write(|mut c| {
            c.displays.remove(&uuid);
            Some(Change::Display(HashSet::from([uuid])))
        })
        .await
    }

    /// Deletes the Playlist with the given Uuid
    ///
    /// Does nothing if no such Playlist is found
    pub async fn delete_playlist(&self, uuid: Uuid) -> Result<(), RedisError> {
        self.write(|mut c| {
            c.playlists.remove(&uuid);
            Some(Change::Playlist(HashSet::from([uuid])))
        })
        .await
    }

    /// Deletes the Schedule with the given Uuid
    ///
    /// Does nothing if no such Schedule is found
    pub async fn delete_schedule(&self, uuid: Uuid) -> Result<(), RedisError> {
        self.write(|mut c| {
            c.schedules.remove(&uuid);
            Some(Change::Schedule(HashSet::from([uuid])))
        })
        .await
    }

    /// Get all PlaylistItem(s) from the active playlist in the display, or the playlist currently active in the display's schedule.
    pub async fn get_display_playlist_items(&self, display: &Uuid) -> Option<Vec<PlaylistItem>> {
        let content = self.read().await;
        match &content.displays.get(display)?.display_material {
            DisplayMaterial::Schedule(uuid) => Some(
                content
                    .playlists
                    .get(&content.schedules.get(uuid)?.playlist)?
                    .items
                    .clone(),
            ),
            DisplayMaterial::Playlist(uuid) => Some(content.playlists.get(uuid)?.items.clone()),
        }
    }

    /// Get Uuids of schedule and playlist connected to Display of given Uuid
    ///
    /// Result is a tuple containing both Uuids as `(Option<schedule_uuid>, playlist_uuid)`.
    /// Playlist is always present if display exists, but schedule can be non if the display is assigned a playlist directly.
    pub async fn get_display_uuids(&self, display: &Uuid) -> Option<(Option<Uuid>, Uuid)> {
        let r = self.read().await;
        match r.displays.get(display)?.display_material {
            DisplayMaterial::Schedule(schedule_uuid) => {
                let playlist_uuid = r.schedules.get(&schedule_uuid)?.playlist;
                Some((Some(schedule_uuid), playlist_uuid))
            }
            DisplayMaterial::Playlist(uuid) => Some((None, uuid)),
        }
    }

    /// Returns String of current state
    pub async fn to_string(&self) -> String {
        format!(
            "{}",
            serde_json::to_string_pretty::<Content>(&*self.content.read().await).unwrap()
        )
    }
}

#[derive(Debug, Clone)]
pub enum Change {
    Display(HashSet<Uuid>),
    Playlist(HashSet<Uuid>),
    /// Notifies a change in the given schedules from, the Api
    /// The correct scheduled playlist may not be set at this time
    ScheduleInput(HashSet<Uuid>),
    /// Sent by the scheduled_loop once it has processed the Schedule
    /// and made sure the correct playlist is set
    Schedule(HashSet<Uuid>),
}
