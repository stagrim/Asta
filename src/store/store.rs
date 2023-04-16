use std::collections::HashMap;

use serde::Deserialize;
use tokio::{fs, sync::{watch::{self, Sender, Receiver}, RwLock, RwLockReadGuard, RwLockWriteGuard}};

type UUID = String;

// TODO: Does currently not update the file when read state is updated. Updates are not synced

#[derive(Deserialize, Debug, Clone)]
pub struct Display {
    pub name: String,
    pub schedule: String
}

#[derive(Deserialize, Debug, Clone)]
pub struct Schedule {
    pub name: String,
    pub playlist: String
}

#[derive(Deserialize, Debug, Clone)]
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

#[derive(Deserialize, Debug, Clone)]
pub struct Content {
    pub displays: HashMap<String, Display>,
    pub playlists: HashMap<String, Playlist>,
    pub schedules: HashMap<String, Schedule>,
}

pub struct Store {
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

        Store { filename, sender, receiver, content }
    }

    async fn read_file(filename: String) -> Content {
        let str = fs::read_to_string(filename).await
            .expect("Could not read json file");

        serde_json::from_str(&str).unwrap()
    }

    pub async fn read(&self) -> RwLockReadGuard<Content> {
        self.content.read().await
    }

    async fn write<F>(&self, fun: F)
    where F: FnOnce(RwLockWriteGuard<Content>) -> () {
        let c = self.content.write().await;
        fun(c);
        self.sender.send(()).expect("Sender dropped");
    }

    /// Updates an existing display to the schedule given.
    /// 
    /// Does not check if the schedule exists or not.
    pub async fn update_display_schedule(&self, display: UUID, schedule: UUID) {
        self.write(|mut c| { 
            c.displays
                .entry(display)
                .and_modify(|d| d.schedule = schedule);
        }).await;
    }

    /// Returns receiver handle to a watch channel which gets notified if store has been updated
    pub fn receiver(&self) -> Receiver<()> {
        self.receiver.clone()
    }

    /// Get all PlaylistItem(s) from the scheduled playlist in the display's schedule
    pub async fn get_display_playlists(&self, display: &UUID) -> Option<Vec<PlaylistItem>> {
        let content = self.read().await;
        let schedule = &content.displays.get(display)?.schedule;
        Some(content.playlists.get(&content.schedules.get(schedule)?.playlist)?.items.clone())
    }

    /// Returns String of current state
    pub async fn to_string(&self) -> String {
        format!("{:#?}", self.content.read().await)
    }
}