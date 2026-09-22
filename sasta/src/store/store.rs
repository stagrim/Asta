use std::collections::{HashMap, HashSet};

use redis::RedisError;
#[cfg(not(test))]
use redis::{Client, JsonAsyncCommands, aio::ConnectionManager};
use serde::{Deserialize, Serialize};

use tokio::sync::broadcast::{self, Receiver, Sender};
use tracing::info;
#[cfg(not(test))]
use tracing::{error_span, warn, warn_span};
use utoipa::ToSchema;
use uuid::Uuid;

use super::schedule::{self, Schedule};

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Display {
    pub name: String,
    pub display_material: DisplayMaterial,
}

// TODO: make this the rust way instead, and do some fallback magic in the Deserialize process instead from the db... Probably much better.
#[derive(Deserialize, Serialize, Debug, ToSchema, Clone)]
#[serde(rename_all = "snake_case")]
#[serde(tag = "type", content = "uuid")]
pub enum DisplayMaterial {
    Schedule(Uuid),
    Playlist(Uuid),
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Playlist {
    pub name: String,
    pub items: Vec<PlaylistItem>,
}

#[derive(Deserialize, Serialize, Debug, Clone, ToSchema)]
#[serde(tag = "type")]
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
    #[serde(rename = "PDF")]
    PortableDocumentFormat {
        #[serde(alias = "name")]
        id: String,
        settings: PDFData,
    },
}

#[derive(Deserialize, Serialize, Debug, Clone, ToSchema)]
pub struct WebsiteData {
    pub url: String,
    pub duration: u64,
}

#[derive(Deserialize, Serialize, Debug, Clone, ToSchema)]
pub struct TextData {
    pub text: String,
    pub duration: u64,
}

#[derive(Deserialize, Serialize, Debug, Clone, ToSchema)]
pub struct ImageData {
    pub src: String,
    pub duration: u64,
}

#[derive(Deserialize, Serialize, Debug, Clone, ToSchema)]
pub struct PDFData {
    pub path: String,
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
    #[cfg(not(test))]
    con: ConnectionManager,
    sender: Sender<Change>,
    pub content: Content,
}

impl Store {
    #[cfg(not(test))]
    pub async fn new(redis_url: &str) -> Self {
        let client = Client::open(redis_url).unwrap();
        let mut con = ConnectionManager::new(client).await.unwrap();
        let (sender, _) = broadcast::channel(5);
        let content = Self::read_file(&mut con).await;

        Store {
            con: con,
            sender,
            content,
        }
    }

    #[cfg(test)]
    pub async fn new(_redis_url: &str) -> Self {
        let (sender, _) = broadcast::channel(5);
        let content = Content {
            displays: HashMap::new(),
            playlists: HashMap::new(),
            schedules: HashMap::new(),
        };

        Store { sender, content }
    }

    #[cfg(not(test))]
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

    /// Returns receiver handle to a watch channel which gets notified if store has been updated
    pub fn receiver(&self) -> Receiver<Change> {
        self.sender.subscribe()
    }

    /// Runs closure with lock write guard handle given as argument
    /// and sends a message signaling a state change once it is done
    #[cfg(not(test))]
    pub async fn write<F>(&mut self, fun: F) -> Result<(), RedisError>
    where
        F: FnOnce(&mut Content) -> Option<Change>,
    {
        let changes = fun(&mut self.content);
        info!("[Store] Sending changes after write: {changes:?}");
        if let Some(c) = changes {
            if let Err(e) = self.sender.send(c) {
                warn!("[Store] No active channels to listen in ({})", e)
            }
        }
        info!("[Store] writing new state to db");
        if let Err(error) = self
            .con
            .json_set::<_, _, _, String>("content", "$", &self.content)
            .await
        {
            error_span!("Redis Error", ?error);
            Err(error)
        } else {
            Ok(())
        }
    }
    #[cfg(test)]
    pub async fn write<F>(&self, _fun: F) -> Result<(), RedisError>
    where
        F: FnOnce(&mut Content) -> Option<Change>,
    {
        Ok(())
    }

    /// Creates a new display
    ///
    /// Overrides existing display with same uuid
    pub async fn create_display(
        &mut self,
        uuid: Uuid,
        name: String,
        display_material: DisplayMaterial,
    ) -> Result<(), RedisError> {
        self.write(|c| {
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
    pub async fn create_playlist(&mut self, uuid: Uuid, name: String) -> Result<(), RedisError> {
        self.write(|c| {
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
        &mut self,
        uuid: Uuid,
        name: String,
        playlist: Uuid,
    ) -> Result<(), RedisError> {
        self.write(|c| {
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
        &mut self,
        uuid: Uuid,
        name: String,
        display_material: DisplayMaterial,
    ) -> Result<(), RedisError> {
        self.write(|c| {
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
        &mut self,
        uuid: Uuid,
        name: String,
        items: Vec<PlaylistItem>,
    ) -> Result<(), RedisError> {
        self.write(|c| {
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
        &mut self,
        uuid: Uuid,
        name: String,
        playlist: Uuid,
        schedules: Vec<schedule::ScheduledPlaylistInput>,
    ) -> Result<(), String> {
        let schedule = match Schedule::new(name, schedules, playlist) {
            Ok(s) => s,
            Err(e) => return Err(e),
        };
        self.write(|c| {
            c.schedules.entry(uuid).and_modify(|s| *s = schedule);
            Some(Change::ScheduleInput(HashSet::from([uuid])))
        })
        .await
        .map_err(|e| e.to_string())
    }

    /// Deletes the display with the given Uuid
    ///
    /// Does nothing if no such display is found
    pub async fn delete_display(&mut self, uuid: Uuid) -> Result<(), RedisError> {
        self.write(|c| {
            c.displays.remove(&uuid);
            Some(Change::Display(HashSet::from([uuid])))
        })
        .await
    }

    /// Deletes the Playlist with the given Uuid
    ///
    /// Does nothing if no such Playlist is found
    pub async fn delete_playlist(&mut self, uuid: Uuid) -> Result<(), RedisError> {
        self.write(|c| {
            c.playlists.remove(&uuid);
            Some(Change::Playlist(HashSet::from([uuid])))
        })
        .await
    }

    /// Deletes the Schedule with the given Uuid
    ///
    /// Does nothing if no such Schedule is found
    pub async fn delete_schedule(&mut self, uuid: Uuid) -> Result<(), RedisError> {
        self.write(|c| {
            c.schedules.remove(&uuid);
            Some(Change::Schedule(HashSet::from([uuid])))
        })
        .await
    }

    /// Get all PlaylistItem(s) from the active playlist in the display, or the playlist currently active in the display's schedule.
    pub async fn get_display_playlist_items(&self, display: &Uuid) -> Option<Vec<PlaylistItem>> {
        let content = &self.content;
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
        match self.content.displays.get(display)?.display_material {
            DisplayMaterial::Schedule(schedule_uuid) => {
                let playlist_uuid = self.content.schedules.get(&schedule_uuid)?.playlist;
                Some((Some(schedule_uuid), playlist_uuid))
            }
            DisplayMaterial::Playlist(uuid) => Some((None, uuid)),
        }
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
