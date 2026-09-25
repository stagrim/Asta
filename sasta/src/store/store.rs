use std::collections::HashMap;

use redis::RedisError;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use tokio::sync::broadcast;
use tracing::warn;
use utoipa::ToSchema;
use uuid::Uuid;

use super::schedule::Schedule;

pub const EVENTS_STREAM: &str = "sasta:events";

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Display {
    pub name: String,
    pub display_material: DisplayMaterial,
}

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

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type", content = "id")]
pub enum Change {
    Display(Uuid),
    Playlist(Uuid),
    Schedule(Uuid),
}

#[derive(Clone)]
pub struct Store {
    client: redis::Client,
}

impl Store {
    pub async fn new(redis_url: &str) -> Self {
        let client = redis::Client::open(redis_url).expect("invalid REDIS_URL");
        let mut connection = client
            .get_multiplexed_async_connection()
            .await
            .expect("connect to Redis for Store initialization");
        let _: Option<String> = redis::cmd("JSON.SET")
            .arg("content")
            .arg(".")
            .arg(r#"{"displays":{},"playlists":{},"schedules":{}}"#)
            .arg("NX")
            .query_async(&mut connection)
            .await
            .expect("initialize the content document");
        Self { client }
    }

    async fn connection(&self) -> Result<redis::aio::MultiplexedConnection, RedisError> {
        self.client.get_multiplexed_async_connection().await
    }

    async fn json_get<T: DeserializeOwned>(
        &self,
        path: impl redis::ToRedisArgs,
    ) -> Result<Option<T>, RedisError> {
        let mut con = self.connection().await?;
        let value: Option<String> = redis::cmd("JSON.GET")
            .arg("content")
            .arg(path)
            .query_async(&mut con)
            .await?;
        value
            .map(|json| serde_json::from_str(&json).map_err(RedisError::from))
            .transpose()
    }

    pub async fn displays(&self) -> Result<HashMap<Uuid, Display>, RedisError> {
        self.json_get(".displays")
            .await
            .map(|value| value.unwrap_or_default())
    }

    pub async fn playlists(&self) -> Result<HashMap<Uuid, Playlist>, RedisError> {
        self.json_get(".playlists")
            .await
            .map(|value| value.unwrap_or_default())
    }

    pub async fn schedules(&self) -> Result<HashMap<Uuid, Schedule>, RedisError> {
        self.json_get(".schedules")
            .await
            .map(|value| value.unwrap_or_default())
    }

    pub async fn display(&self, uuid: Uuid) -> Result<Option<Display>, RedisError> {
        self.json_get(format!(".displays.{uuid}")).await
    }

    pub async fn playlist(&self, uuid: Uuid) -> Result<Option<Playlist>, RedisError> {
        self.json_get(format!(".playlists.{uuid}")).await
    }

    pub async fn schedule(&self, uuid: Uuid) -> Result<Option<Schedule>, RedisError> {
        self.json_get(format!(".schedules.{uuid}")).await
    }

    async fn mutate(
        &self,
        path: String,
        value: Option<&impl Serialize>,
        change: Change,
    ) -> Result<(), RedisError> {
        let mut con = self.connection().await?;
        let event = serde_json::to_string(&change).map_err(RedisError::from)?;

        let mut pipe = redis::pipe();
        pipe.atomic();
        match value {
            Some(value) => {
                pipe.cmd("JSON.SET")
                    .arg("content")
                    .arg(path)
                    .arg(serde_json::to_string(value).map_err(RedisError::from)?);
            }
            None => {
                pipe.cmd("JSON.DEL").arg("content").arg(path);
            }
        }
        pipe.cmd("XADD")
            .arg(EVENTS_STREAM)
            .arg("*")
            .arg("change")
            .arg(event);
        pipe.query_async::<()>(&mut con).await
    }

    pub async fn create_display(&self, uuid: Uuid, display: Display) -> Result<(), RedisError> {
        self.mutate(
            format!(".displays.{uuid}"),
            Some(&display),
            Change::Display(uuid),
        )
        .await
    }

    pub async fn create_playlist(&self, uuid: Uuid, playlist: Playlist) -> Result<(), RedisError> {
        self.mutate(
            format!(".playlists.{uuid}"),
            Some(&playlist),
            Change::Playlist(uuid),
        )
        .await
    }

    pub async fn create_schedule(&self, uuid: Uuid, schedule: Schedule) -> Result<(), RedisError> {
        self.mutate(
            format!(".schedules.{uuid}"),
            Some(&schedule),
            Change::Schedule(uuid),
        )
        .await
    }

    pub async fn update_display(&self, uuid: Uuid, display: Display) -> Result<(), RedisError> {
        self.create_display(uuid, display).await
    }

    pub async fn update_playlist(&self, uuid: Uuid, playlist: Playlist) -> Result<(), RedisError> {
        self.create_playlist(uuid, playlist).await
    }

    pub async fn update_schedule(&self, uuid: Uuid, schedule: Schedule) -> Result<(), RedisError> {
        self.create_schedule(uuid, schedule).await
    }

    pub async fn delete_display(&self, uuid: Uuid) -> Result<(), RedisError> {
        self.mutate(
            format!(".displays.{uuid}"),
            None::<&Display>,
            Change::Display(uuid),
        )
        .await
    }

    pub async fn delete_playlist(&self, uuid: Uuid) -> Result<(), RedisError> {
        self.mutate(
            format!(".playlists.{uuid}"),
            None::<&Playlist>,
            Change::Playlist(uuid),
        )
        .await
    }

    pub async fn delete_schedule(&self, uuid: Uuid) -> Result<(), RedisError> {
        self.mutate(
            format!(".schedules.{uuid}"),
            None::<&Schedule>,
            Change::Schedule(uuid),
        )
        .await
    }

    pub async fn get_display_playlist_items(
        &self,
        display_uuid: &Uuid,
    ) -> Option<Vec<PlaylistItem>> {
        let display = self.display(*display_uuid).await.ok()??;
        let playlist_uuid = match display.display_material {
            DisplayMaterial::Schedule(schedule_uuid) => {
                let schedule = self.schedule(schedule_uuid).await.ok()??;
                schedule.current_playlist(&chrono::Local::now())
            }
            DisplayMaterial::Playlist(playlist_uuid) => playlist_uuid,
        };
        self.playlist(playlist_uuid).await.ok()??.items.into()
    }

    pub async fn get_display_uuids(&self, display_uuid: &Uuid) -> Option<(Option<Uuid>, Uuid)> {
        let display = self.display(*display_uuid).await.ok()??;
        match display.display_material {
            DisplayMaterial::Schedule(schedule_uuid) => {
                let schedule = self.schedule(schedule_uuid).await.ok()??;
                Some((
                    Some(schedule_uuid),
                    schedule.current_playlist(&chrono::Local::now()),
                ))
            }
            DisplayMaterial::Playlist(playlist_uuid) => Some((None, playlist_uuid)),
        }
    }

    pub fn client(&self) -> redis::Client {
        self.client.clone()
    }
}

pub async fn redis_event_listener(client: redis::Client, sender: broadcast::Sender<Change>) {
    loop {
        let mut con = match client.get_multiplexed_async_connection().await {
            Ok(con) => con,
            Err(error) => {
                warn!("Could not connect Redis event listener: {error}; retrying");
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                continue;
            }
        };

        let mut last_id = String::from("0-0");

        loop {
            let result: Result<redis::streams::StreamReadReply, RedisError> = redis::cmd("XREAD")
                .arg("BLOCK")
                .arg(30_000)
                .arg("STREAMS")
                .arg(EVENTS_STREAM)
                .arg(&last_id)
                .query_async(&mut con)
                .await;

            match result {
                Ok(reply) => {
                    for stream in reply.keys {
                        for entry in stream.ids {
                            last_id = entry.id;

                            if let Some(payload) = entry.map.get("change") {
                                if let Ok(json) = redis::from_redis_value::<String>(payload.clone())
                                {
                                    if let Ok(change) = serde_json::from_str::<Change>(&json) {
                                        let _ = sender.send(change);
                                    }
                                }
                            }
                        }
                    }
                }

                Err(error) if error.is_timeout() => {
                    // No event arrived during BLOCK. This is expected.
                    continue;
                }

                Err(error) => {
                    warn!("Redis event listener connection lost: {error}");
                    break;
                }
            }
        }

        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    }
}
