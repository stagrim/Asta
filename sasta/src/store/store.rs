use std::collections::{HashMap, HashSet};

use chrono::Local;
use serde::{Deserialize, Serialize};
use tokio::{sync::{broadcast::{self, Sender, Receiver}, RwLock, RwLockReadGuard, RwLockWriteGuard, oneshot}, time::{sleep_until, Instant}};
use ts_rs::TS;
use uuid::Uuid;

use super::schedule::{Schedule, Moment, self};

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Display {
    pub name: String,
    pub schedule: Uuid
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Playlist {
    pub name: String,
    pub items: Vec<PlaylistItem>
}

#[derive(Deserialize, Serialize, Debug, Clone, TS)]
#[serde(tag = "type")]
#[ts(export, export_to = "api_bindings/update/")]
pub enum PlaylistItem {
    #[serde(rename = "WEBSITE")]
    Website { name: String, settings: WebsiteData },
    #[serde(rename = "TEXT")]
    Text { name: String, settings: TextData },
    #[serde(rename = "IMAGE")]
    Image { name: String, settings: ImageData }
}

#[derive(Deserialize, Serialize, Debug, Clone, TS)]
#[ts(export, export_to = "api_bindings/update/")]
pub struct WebsiteData {
    pub url: String,
    pub duration: u64
}

#[derive(Deserialize, Serialize, Debug, Clone, TS)]
#[ts(export, export_to = "api_bindings/update/")]
pub struct TextData {
    pub text: String,
    pub duration: u64
}

#[derive(Deserialize, Serialize, Debug, Clone, TS)]
#[ts(export, export_to = "api_bindings/update/")]
pub struct ImageData {
    pub src: String,
    pub duration: u64
}

#[derive(Deserialize, Serialize, Debug)]
pub struct Content {
    pub displays: HashMap<Uuid, Display>,
    pub playlists: HashMap<Uuid, Playlist>,
    pub schedules: HashMap<Uuid, Schedule>,
}

pub struct Store {
    filename: String,
    sender: Sender<Change>,
    content: RwLock<Content>
}

impl Store {
    pub async fn new() -> Self {
        let filename = String::from("content.json");
        let (sender, _) = broadcast::channel(5);
        let content = RwLock::new(Self::read_file(filename.clone()).await);

        let s = Store { filename, sender, content };
        s
    }

    async fn read_file(filename: String) -> Content {
        // let mut file = OpenOptions::new()
        //     .read(true)
        //     .write(true)
        //     .create(true)
        //     .open(&filename)
        //     .await.unwrap();
        
        // let mut str = String::new();
        let str = tokio::fs::read_to_string(filename).await
            .expect("[Store] Could not read json file");

        match serde_json::from_str(&str) {
            Ok(c) => c,
            Err(_) => {
                println!("[Store] could not parse file content, starting with a blank state");
                Content {
                    displays: HashMap::new(),
                    playlists: HashMap::new(),
                    schedules: HashMap::new(),
                }
            },
        }
    }

    /// Updates all Schedules' Playlists to the (schedule_uuid, active_playlist_uuid) pairs
    async fn update_schedule_active_playlist(&self, vec: Vec<(Uuid, Uuid)>) {
        // Exit if schedule does not exists or if it is already set to the given playlist 
        // if !self.read().await.schedules.contains_key(&schedule) || self.read().await.schedules.get(&schedule).unwrap().playlist == active_playlist {
        //     return;
        // }

        self.write(|mut c| {
            vec.iter().for_each( |(schedule, playlist)| {
                c.schedules
                    .entry(*schedule)
                    .and_modify(|s| s.playlist = *playlist);
            });
            Some(Change::Schedule(HashSet::from_iter(vec.iter().map(|v| v.0))))
        }).await;
    }

    /// Starts scheduling loop updating the active playlists when necessary
    /// 
    /// Cancels sent token when state has been updated to the active scheduled playlists
    pub async fn schedule_loop(&self, tx: oneshot::Sender<()>) {
        let mut current_moment = Local::now();
        let mut receiver = self.receiver();

        let schedules: Vec<(Uuid, Schedule)> = self.read().await.schedules
            .iter()
            .map(|(schedule_uuid, schedule)| (schedule_uuid.clone(), schedule.clone()))
            .collect();

        // Updates all schedules to their current active scheduled playlist
        if !schedules.is_empty() {
            self.write(|mut c| {
                schedules.iter().for_each(|(uuid, schedule)| {
                    c.schedules
                        .entry(*uuid)
                        .and_modify(|s| s.playlist = schedule.current_playlist(&current_moment));
                });
                // Change notice not needed since main thread waits on oneshot notice before continuing
                None
            }).await;
            println!("[Scheduler] Updated Schedules to current active playlist");
        }

        // Notify oneshot channel that schedules have been updated to active playlists
        if let Err(_) = tx.send(()) {
            println!("[Scheduler] Could not notify listener, sender dropped");
        }

        'main: loop {
            let instant = Instant::now();
            let schedules: Vec<(Uuid, Schedule)> = self.read().await.schedules
                .iter()
                .map(|(schedule_uuid, schedule)| (schedule_uuid.clone(), schedule.clone()))
                .collect();
            
            let mut moments: Vec<(Uuid, Moment)> = schedules.iter()
                .filter_map(|(schedule_uuid, schedule)| match schedule.next_schedule(&current_moment) {
                    Some(m) => Some((schedule_uuid.clone(), m)),
                    None => None,
                    
                })
                .collect();
            
            if moments.is_empty() {
                println!("[Scheduler] No loaded Schedule has any scheduled playlists, waiting on an update to a Schedule...");
                loop {
                    match receiver.recv().await {
                        Ok(Change::ScheduleInput(uuids)) => {
                            let read = self.read().await;
                            if uuids.iter().any(|u| read.schedules.get(u).is_some_and(|s| s.has_scheduled_playlists())) {
                                println!("[Scheduler] An updated Schedule has scheduled playlists, rerunning loop");
                                break
                            }
                        },
                        Err(e) => println!("[Scheduler] RecvError: {e}"),
                        _ => println!("[Scheduler] Non relevant change received, continue waiting"),
                    }
                }
                continue
            }

            
            let closest_time = moments.iter()
                .min_by_key(|(_, m)| m.time).unwrap().1.time;

            moments = moments.into_iter().filter(|(_, m)| m.time == closest_time).collect();

            let sleep = match (closest_time - Local::now()).to_std() {
                Ok(d) => instant + d,
                Err(_) => instant,
            };

            println!("[Scheduler] Sleeping for {:?} until {} to change active playlists", sleep.duration_since(instant), closest_time.to_string());

            loop {
                tokio::select! {
                    _ = sleep_until(sleep) => {
                        println!("[Scheduler] Breaking");
                        break
                    },
                    change = receiver.recv() => {
                        match change {
                            //TODO: When updating a schedule, the new schedule is overridden in the API, and since the 'set current block' lies before the loop, they are never reverted to the present version
                            Ok(Change::ScheduleInput(uuids)) => {
                                println!("[Scheduler] Schedules updated, rerunning loop");
                                self.write(|mut c| {
                                    uuids.iter().for_each(|uuid| {
                                        c.schedules
                                            .entry(*uuid)
                                            .and_modify(|s| s.playlist = s.current_playlist(&current_moment));
                                    });
                                    Some(Change::Schedule(uuids))
                                }).await;
                                continue 'main
                            },
                            Err(e) => println!("[Scheduler] RecvError: {e}"),
                            _ => println!("[Scheduler] Non relevant change received, continue waiting"),
                        }
                    },
                }
            }
            println!("[Scheduler] Sleep done, updating active playlists");
            
            self.update_schedule_active_playlist(
                moments.iter()
                    .map(|(u, m)| (*u, m.playlist))
                    .inspect(|(uuid, _)| println!("[Scheduler] Updating Schedule {uuid} active playlist"))
                    .collect::<Vec<_>>()
            ).await;
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
    async fn write<F>(&self, fun: F)
    where F: FnOnce(RwLockWriteGuard<Content>) -> Option<Change> {
        let c = self.content.write().await;
        let changes = fun(c);
        println!("[Store] Sending changes after write: {changes:?}");
        if let Some(c) = changes {
            if let Err(e) = self.sender.send(c) {
                println!("[Store] No active channels to listen in ({})", e)
            }
        }

        println!("[Store] writing new state to file");
        if let Err(_) = tokio::fs::write(&self.filename, self.to_string().await).await {
            println!("[Store] Error writing state to file after update, log updated state instead: \n{}", self.to_string().await);
        }
    }

    /// Creates a new display
    /// 
    /// Overrides existing display with same uuid
    pub async fn create_display(&self, uuid: Uuid, name: String, schedule: Uuid) {
        self.write(|mut c| {
            c.displays.insert(uuid, Display { name, schedule });
            Some(Change::Display(HashSet::from([uuid])))
        }).await;
    }

    /// Creates a new playlist
    /// 
    /// Overrides existing playlist with same uuid
    pub async fn create_playlist(&self, uuid: Uuid, name: String) {
        self.write(|mut c| {
            c.playlists.insert(uuid, Playlist { name, items: vec![] });
            Some(Change::Playlist(HashSet::from([uuid])))
        }).await;
    }

    /// Creates a new Schedule
    /// 
    /// Overrides existing Schedule with same uuid
    pub async fn create_schedule(&self, uuid: Uuid, name: String, playlist: Uuid) {
        self.write(|mut c| {
            c.schedules.insert(uuid, Schedule::new(name, vec![], playlist).unwrap());
            Some(Change::Schedule(HashSet::from([uuid])))
        }).await;
    }

    /// Updates the display with the given Uuid
    /// 
    /// Does nothing if no such display is found
    pub async fn update_display(&self, uuid: Uuid, name: String, schedule: Uuid) {
        self.write(|mut c| {
            c.displays
                .entry(uuid)
                .and_modify(|d| *d = Display { name, schedule });
            Some(Change::Display(HashSet::from([uuid])))
        }).await
    }

    /// Updates the playlist with the given Uuid
    /// 
    /// Does nothing if no such playlist is found
    pub async fn update_playlist(&self, uuid: Uuid, name: String, items: Vec<PlaylistItem>) {
        self.write(|mut c| {
            c.playlists
                .entry(uuid)
                .and_modify(|p| *p = Playlist { name, items });
            Some(Change::Playlist(HashSet::from([uuid])))
        }).await
    }

    /// Updates the Schedule with the given Uuid
    /// 
    /// Does nothing if no such Schedule is found
    pub async fn update_schedule(&self, uuid: Uuid, name: String, playlist: Uuid, schedules: Vec<schedule::ScheduledPlaylistInput>) -> Result<(), String> {
        let schedule = match Schedule::new(name, schedules, playlist) {
            Ok(s) => s,
            Err(e) => return Err(e),
        };
        self.write(|mut c| {
            c.schedules
                .entry(uuid)
                .and_modify(|s| *s = schedule);
            Some(Change::ScheduleInput(HashSet::from([uuid])))
        }).await;
        Ok(())
    }

    /// Deletes the display with the given Uuid
    /// 
    /// Does nothing if no such display is found
    pub async fn delete_display(&self, uuid: Uuid) {
        self.write(|mut c| {
            c.displays.remove(&uuid);
            Some(Change::Display(HashSet::from([uuid])))
        }).await
    }

    /// Deletes the Playlist with the given Uuid
    /// 
    /// Does nothing if no such Playlist is found
    pub async fn delete_playlist(&self, uuid: Uuid) {
        self.write(|mut c| {
            c.playlists.remove(&uuid);
            Some(Change::Playlist(HashSet::from([uuid])))
        }).await
    }

    /// Deletes the Schedule with the given Uuid
    /// 
    /// Does nothing if no such Schedule is found
    pub async fn delete_schedule(&self, uuid: Uuid) {
        self.write(|mut c| {
            c.schedules.remove(&uuid);
            Some(Change::Schedule(HashSet::from([uuid])))
        }).await
    }

    /// Get all PlaylistItem(s) from the scheduled playlist in the display's schedule
    pub async fn get_display_playlists(&self, display: &Uuid) -> Option<Vec<PlaylistItem>> {
        let content = self.read().await;
        let schedule = &content.displays.get(display)?.schedule;
        Some(content.playlists.get(&content.schedules.get(schedule)?.playlist)?.items.clone())
    }

    /// Get Uuids of schedule and playlist connected to Display of given Uuid
    /// 
    /// Result is a tuple containing both Uuids as `(schedule_uuid, playlist_uuid)`
    pub async fn get_display_uuids(&self, display: &Uuid) -> Option<(Uuid, Uuid)> {
        let r = self.read().await;
        let schedule = r.displays.get(display)?.schedule;
        let playlist = r.schedules.get(&schedule)?.playlist;
        Some((schedule, playlist))
    }

    /// Returns String of current state
    pub async fn to_string(&self) -> String {
        format!("{}", serde_json::to_string_pretty::<Content>(&*self.content.read().await).unwrap())
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
