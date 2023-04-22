use std::{collections::{HashMap, HashSet}, time::Duration};

use chrono::Local;
use serde::Deserialize;
use tokio::{fs, sync::{broadcast::{self, Sender, Receiver}, RwLock, RwLockReadGuard, RwLockWriteGuard, oneshot}, time::sleep};
use uuid::Uuid;

use super::schedule::{Schedule, Moment};

#[derive(Deserialize, Debug, Clone)]
pub struct Display {
    pub name: String,
    pub schedule: Uuid
}

#[derive(Deserialize, Debug)]
pub struct Playlist {
    pub name: String,
    pub items: Vec<PlaylistItem>
}

#[derive(Deserialize, Debug, Clone)]
#[serde(tag = "type")]
pub enum PlaylistItem {
    #[serde(rename(deserialize = "WEBSITE"))]
    Website { name: String, settings: WebsiteData },
    #[serde(rename(deserialize = "TEXT"))]
    Text { name: String, settings: TextData },
    #[serde(rename(deserialize = "IMAGE"))]
    Image { name: String, settings: ImageData }
}

#[derive(Deserialize, Debug, Clone)]
pub struct WebsiteData {
    pub url: String,
    pub duration: u64
}

#[derive(Deserialize, Debug, Clone)]
pub struct TextData {
    pub text: String,
    pub duration: u64
}

#[derive(Deserialize, Debug, Clone)]
pub struct ImageData {
    pub src: String,
    pub duration: u64
}

#[derive(Deserialize, Debug)]
pub struct Content {
    pub displays: HashMap<Uuid, Display>,
    pub playlists: HashMap<Uuid, Playlist>,
    pub schedules: HashMap<Uuid, Schedule>,
}

pub struct Store {
    // TODO: Does currently not update the file when read state is updated. Updates are not synced
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
        let str = fs::read_to_string(filename).await
            .expect("[Store] Could not read json file");

        serde_json::from_str(&str).unwrap()
    }

    /// Get all PlaylistItem(s) from the scheduled playlist in the display's schedule
    async fn update_schedule_active_playlist(&self, schedule: Uuid, active_playlist: Uuid) {
        // Exit if schedule does not exists or if it is already set to the given playlist 
        // if !self.read().await.schedules.contains_key(&schedule) || self.read().await.schedules.get(&schedule).unwrap().playlist == active_playlist {
        //     return;
        // }

        self.write(|mut c| {
            c.schedules
                .entry(schedule)
                .and_modify(|s| s.playlist = active_playlist);
            Change::Schedule(HashSet::from([schedule]))
        }).await;
    }

    /// Starts scheduling loop updating the active playlists when necessary
    /// 
    /// Cancels sent token when state has been updated to the active scheduled playlists
    pub async fn schedule_loop(&self, tx: oneshot::Sender<()>) {
        let mut current_moment = Local::now();

        let schedules: Vec<(Uuid, Schedule)> = self.read().await.schedules
            .iter()
            .map(|(schedule_uuid, schedule)| (schedule_uuid.clone(), schedule.clone()))
            .collect();

        for (uuid, schedule) in &schedules {
            self.update_schedule_active_playlist(uuid.clone(), schedule.current_playlist(&current_moment)).await;
        }

        if let Err(_) = tx.send(()) {
            println!("[Scheduler] Could not notify listener, sender dropped");
        }

        loop {
            let mut moments: Vec<(Uuid, Moment)> = schedules.iter()
                .filter_map(|(schedule_uuid, schedule)| match schedule.next_schedule(&current_moment) {
                    Some(m) => Some((schedule_uuid.clone(), m)),
                    None => None,
                    
                })
                .collect();

            if moments.is_empty() {
                // TODO: listen for changes to schedule and restart process
                println!("[Scheduler] Well, since updating schedules while running is not implemented yet, schedule thread will exit as no future moments was found. Bye!");
                break;
            }

            
            let closest_time = moments.iter()
                .min_by_key(|(_, m)| m.time).unwrap().1.time;

            moments = moments.into_iter().filter(|(_, m)| m.time == closest_time).collect();

            let sleep = match (closest_time - current_moment).to_std() {
                Ok(d) => sleep(d),
                Err(_) => sleep(Duration::from_secs(0)),
            };

            println!("[Scheduler] Sleeping until {} to change active playlists", closest_time.to_string());

            sleep.await;

            for (uuid, moment) in moments {
                println!("[Scheduler] Updated schedule {uuid} active playlist");
                self.update_schedule_active_playlist(uuid.clone(), moment.playlist).await;
            }
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
    where F: FnOnce(RwLockWriteGuard<Content>) -> Change {
        let c = self.content.write().await;
        let changes = fun(c);
        println!("[Store] Sending changes after write: {changes:?}");
        if let Err(e) = self.sender.send(changes) {
            println!("[Store] No active channels to listen in ({})", e)
        }
    }

    /// Updates an existing display to the schedule given.
    /// 
    /// Does not check if the schedule exists or not.
    pub async fn update_display_schedule(&self, display: Uuid, schedule: Uuid) {
        self.write(|mut c| {
            c.displays
                .entry(display)
                .and_modify(|d| d.schedule = schedule);
            Change::Display(HashSet::from([display]))
        }).await;
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
        format!("{:#?}", self.content.read().await)
    }
}

#[derive(Debug, Clone)]
pub enum Change {
    Display(HashSet<Uuid>),
    Playlist(HashSet<Uuid>),
    Schedule(HashSet<Uuid>),
}