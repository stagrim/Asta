use std::collections::HashMap;

use serde::Deserialize;
use serde_json::Value;
use tokio::fs;

#[derive(Deserialize, Debug)]
pub struct Display {
    name: String,
    schedule: String
}

#[derive(Deserialize, Debug)]
pub struct Schedule {
    name: String,
    playlist: String
}

#[derive(Deserialize, Debug)]
pub struct Playlist {
    name: String,
    items: Vec<PlaylistItem>
}

#[derive(Deserialize, Debug)]
#[serde(tag = "type")]
pub enum PlaylistItem {
    #[serde(rename(deserialize = "WEBSITE"))]
    Website { name: String, settings: WebsiteData },
    // #[serde(rename(deserialize = "TEXT"))]
    // Text { name: String, settings: WebsiteData },
    // #[serde(rename(deserialize = "IMAGE"))]
    // Image { name: String, settings: WebsiteData }
}

#[derive(Deserialize, Debug)]
pub struct WebsiteData {
    url: String,
    duration: u32
}

#[derive(Deserialize, Debug)]
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

        let content: Content = serde_json::from_str(&str).unwrap();

        content
    }
}
