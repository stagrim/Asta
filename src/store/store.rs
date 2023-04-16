use std::collections::HashMap;

use serde::Deserialize;
use tokio::fs;

// TODO: Move all state logic here, and replace Watch with RwLock and Watch to only signal a change.
// Pass around a Content or Store instance to create consumers?

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
    filename: String
}

impl Store {
    pub fn new() -> Self {
        Store {
            filename: String::from("content.json")
        }
    }

    pub async fn load(&self) -> Content {
        let str = fs::read_to_string(&self.filename).await
            .expect("Could not read json file");

        serde_json::from_str(&str).unwrap()
    }
}
