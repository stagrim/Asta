use std::{collections::HashMap, time::Duration};

use chrono::Local;
use serde::Deserialize;
use tokio::{fs, sync::{watch::{self, Sender, Receiver}, RwLock, RwLockReadGuard, RwLockWriteGuard}, time::sleep};
use tokio_util::sync::CancellationToken;
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
    sender: Sender<()>,
    receiver: Receiver<()>,
    content: RwLock<Content>
}

impl Store {
    pub async fn new() -> Self {
        let filename = String::from("content.json");
        let (sender, receiver) = watch::channel(());
        let content = RwLock::new(Self::read_file(filename.clone()).await);

        let s = Store { filename, sender, receiver, content };
        s
    }

    async fn read_file(filename: String) -> Content {
        let str = fs::read_to_string(filename).await
            .expect("Could not read json file");

        serde_json::from_str(&str).unwrap()
    }

    /// Get all PlaylistItem(s) from the scheduled playlist in the display's schedule
    async fn update_schedule_active_playlist(&self, schedule: Uuid, active_playlist: Uuid) {
        self.write(|mut c| {
            c.schedules
                .entry(schedule)
                .and_modify(|s| s.playlist = active_playlist);
        }).await;
    }

    // TODO: listen for changes to schedule and restart process. Also cache and check if updates are necessary to avoid notifying all
    // listeners of a change which didn't update anything
    pub async fn schedules(&self, done: CancellationToken) {
        let mut current_moment = Local::now();

        let schedules: Vec<(Uuid, Schedule)> = self.read().await.schedules
            .iter()
            .map(|(schedule_uuid, schedule)| (schedule_uuid.clone(), schedule.clone()))
            .collect();

        for (uuid, schedule) in &schedules {
            self.update_schedule_active_playlist(uuid.clone(), schedule.current_playlist(&current_moment)).await;
        }

        done.cancel();

        loop {
            let mut moments: Vec<(Uuid, Moment)> = schedules.iter()
                .filter_map(|(schedule_uuid, schedule)| match schedule.next_schedule(&current_moment) {
                    Some(m) => Some((schedule_uuid.clone(), m)),
                    None => None,
                    
                })
                .collect();

            if moments.is_empty() {
                println!("Well, since updating schedules while running is not implemented yet, schedule thread will exit as no future moments was found. Bye!");
                break;
            }

            
            let closest_time = moments.iter()
                .min_by_key(|(_, m)| m.time).unwrap().1.time;

            moments = moments.into_iter().filter(|(_, m)| m.time == closest_time).collect();

            let sleep = match (closest_time - current_moment).to_std() {
                Ok(d) => sleep(d),
                Err(_) => sleep(Duration::from_secs(0)),
            };

            println!("Sleeping until {} to change active playlists", closest_time.to_string());

            sleep.await;

            for (uuid, moment) in moments {
                println!("Updated schedule {uuid} active playlist");
                self.update_schedule_active_playlist(uuid.clone(), moment.playlist).await;
            }
            current_moment = closest_time;
        }
    }

    /// Returns receiver handle to a watch channel which gets notified if store has been updated
    pub fn receiver(&self) -> Receiver<()> {
        self.receiver.clone()
    }

    pub async fn read(&self) -> RwLockReadGuard<Content> {
        self.content.read().await
    }

    /// Runs closure with lock write guard handle given as argument
    /// and sends a message signalling a state change once it is done
    async fn write<F>(&self, fun: F)
    where F: FnOnce(RwLockWriteGuard<Content>) -> () {
        let c = self.content.write().await;
        fun(c);
        self.sender.send(()).expect("Channel closed");
    }

    /// Updates an existing display to the schedule given.
    /// 
    /// Does not check if the schedule exists or not.
    pub async fn update_display_schedule(&self, display: Uuid, schedule: Uuid) {
        self.write(|mut c| { 
            c.displays
                .entry(display)
                .and_modify(|d| d.schedule = schedule);
        }).await;
    }

    /// Get all PlaylistItem(s) from the scheduled playlist in the display's schedule
    pub async fn get_display_playlists(&self, display: &Uuid) -> Option<Vec<PlaylistItem>> {
        let content = self.read().await;
        let schedule = &content.displays.get(display)?.schedule;
        Some(content.playlists.get(&content.schedules.get(schedule)?.playlist)?.items.clone())
    }

    /// Returns String of current state
    pub async fn to_string(&self) -> String {
        format!("{:#?}", self.content.read().await)
    }
}