
use std::{net::SocketAddr, sync::{Arc, self}, time::Duration, fmt::Display};

use axum::extract::ws::{Message, WebSocket};
use futures_util::{SinkExt, stream::{SplitSink, SplitStream}, StreamExt};
use serde::{Deserialize, Serialize};
use tokio::{sync::Mutex, time::{timeout, Instant, sleep_until}};
use tracing::{info, error, warn, trace};
use uuid::Uuid;

use crate::store::store::{PlaylistItem, Store, Change, TextData, WebsiteData, ImageData};

#[derive(Deserialize, Clone)]
struct HelloRequest {
    uuid: Uuid,
    hostname: String
}

#[derive(Debug, Serialize)]
pub enum Payload {
    #[serde(rename(serialize = "display"))]
    Display(DisplayPayload),
    #[serde(rename(serialize = "name"))]
    Name(String),
    #[serde(rename(serialize = "pending"))]
    Pending(bool)
}

#[derive(Serialize, Debug, Clone)]
#[serde(tag = "type")]
pub enum DisplayPayload {
    #[serde(rename(serialize = "WEBSITE"))]
    Website { data: WebsitePayload },
    #[serde(rename(serialize = "TEXT"))]
    Text { data: WebsitePayload },
    #[serde(rename(serialize = "IMAGE"))]
    Image { data: WebsitePayload }
}

#[derive(Serialize, Debug, Clone)]
pub struct WebsitePayload {
    pub content: String
}

pub async fn client_connection(socket: WebSocket, who: SocketAddr, store: Arc<Store>) {
    let (client_send, mut client_receive) = socket.split();
    let client_send = Arc::new(Mutex::new(client_send));

    // Wait for a hello response from connected client to get its UUID
    let HelloRequest { uuid: client_uuid, hostname: client_hostname } = loop {
        if let Some(Ok(Message::Text(msg))) = client_receive.next().await {
            match serde_json::from_str(&msg) {
                Ok(msg) => break msg,
                Err(_) => error!("[{who}] {msg:?} was not a HelloRequest")
            };
        }
    };

    let client_name = Arc::new(sync::RwLock::new(None));

    info!("[{who}] Connected with provided Uuid '{client_uuid}' and hostname '{client_hostname}'");

    let mut heartbeat_handle = tokio::spawn(heartbeat(client_send.clone(), client_receive, who, client_name.clone()));

    let mut client_handle = tokio::spawn(async move {

        let mut rx = store.receiver();
        loop {
            let display_option = store.read().await.displays.get(&client_uuid).and_then(|d| Some(d.clone()));
            match display_option {
                Some(d) => {
                    let mut w = client_name.write().unwrap();
                    *w = Some(d.name);
                    break;
                },
                None => {
                    warn!("[{who}] No screen defined with UUID {:?}, waiting for Display update", client_uuid);
                    let msg = Message::Text(serde_json::to_string(&Payload::Pending(true)).unwrap());
                    client_send.lock().await.send(msg).await.unwrap();

                    // Wait until display is updated change, then try again
                    loop {
                        match rx.recv().await.unwrap() {
                            Change::Display(m) if m.contains(&client_uuid) => break,
                            _ => ()
                        };
                    }
                    continue;
                },
            }
        };

        let client_name = client_name.read().unwrap().clone().unwrap_or("Got a None".to_string());

        let msg = Message::Text(serde_json::to_string(&Payload::Name(client_name.clone())).unwrap());
        client_send.lock().await.send(msg).await.unwrap();

        info!("[{who}] was given the name {client_name}");

        // outer loop collects the PlaylistItems(s) before entering the repeating send loop
        'outer_send_loop: loop {
            let (schedule_uuid, playlist_uuid) = store.get_display_uuids(&client_uuid).await.unwrap();
            let mut playlist = match store.get_display_playlists(&client_uuid).await {
                Some(p) => p,
                None => {
                    error!("[{who} ({client_name})] Error: Display playlist could not be found");
                    return
                },
            };

            // If playlist is empty, add text stating such to display loop
            if playlist.is_empty() {
                playlist.push(PlaylistItem::Text { name: "pending".into(), settings: TextData { text: "No Playlist added".into(), duration: 0 } });
            }

            for item in playlist.into_iter().cycle() {
                let sleep_duration;
                let payload = match item {
                    PlaylistItem::Website { name, settings: WebsiteData { url, duration } } => {
                        info!("[{who} ({client_name})] Sending Website '{name}' with url '{}'", url);
                        sleep_duration = duration;
                        DisplayPayload::Website { data: WebsitePayload { content: url } }
                    },
                    PlaylistItem::Text { name, settings: TextData { text, duration } } => {
                        info!("[{who} ({client_name})] Sending Text '{name}' with text '{text}'");
                        sleep_duration = duration;
                        DisplayPayload::Text { data: WebsitePayload { content: text } }
                    },
                    PlaylistItem::Image { name, settings: ImageData { src, duration } } => {
                        info!("[{who} ({client_name})] Sending Image '{name}' with src '{src}'");
                        sleep_duration = duration;
                        DisplayPayload::Image { data: WebsitePayload { content: src } }
                    },
                    PlaylistItem::BackgroundAudio { .. } => todo!(),

                };

                let msg = Message::Text(serde_json::to_string(&Payload::Display(payload)).unwrap());
                client_send.lock().await.send(msg).await.unwrap();

                let now = Instant::now();
                // Sleep for an infinite time if duration of the PlaylistItem is zero.
                // Maybe not "infinite" in a literal sense, but at least for some billion years,
                // which I would consider good enough.
                // A bug that needs to be fixed is that this will stop working in about hundred billion years
                // and the thread will panic with a overflow error. Note that this will happen for both the
                // debug and release builds. Truly no one is safe from the catastrophe that will occur...
                let sleep = now + Duration::from_secs(if sleep_duration == 0 {
                    u64::MAX / 10 as u64
                } else {
                    sleep_duration
                });

                loop {
                    info!("[{who} ({client_name})] Sleeping for {} seconds", (sleep - now).as_secs());
                    tokio::select! {
                        _ = sleep_until(sleep) => break,
                        notification = rx.recv() => {
                            match notification {
                                Ok(c) => {
                                    match c {
                                        Change::Display(d) if d.contains(&client_uuid) => {
                                            info!("[{who} ({client_name})] Display {} has changed, restarting send loop", client_uuid);
                                            continue 'outer_send_loop
                                        },
                                        Change::Playlist(p) if p.contains(&playlist_uuid)  => {
                                            info!("[{who} ({client_name})] Playlist {} has changed, restarting send loop", playlist_uuid);
                                            continue 'outer_send_loop
                                        },
                                        Change::Schedule(s) if s.contains(&schedule_uuid) => {
                                            info!("[{who} ({client_name})] Schedule {} has changed, restarting send loop", schedule_uuid);
                                            continue 'outer_send_loop
                                        },
                                        _ => {
                                            trace!("[{who} ({client_name})] Message received but no Change relate to current client, skipping")
                                        }
                                    }
                                },
                                Err(e) => {
                                    error!("[{who} ({client_name})] Exit thread due to error: {e}");
                                    return;
                                },
                            }
                        }
                    }
                }
            }
        }
    });

    tokio::select! {
        _ = &mut heartbeat_handle  => client_handle.abort(),
        _ = &mut client_handle     => heartbeat_handle.abort(),
    };

    info!("[{who}] Disconnected from client!");
}


async fn heartbeat(
    sender: Arc<Mutex<SplitSink<WebSocket, Message>>>,
    mut receiver: SplitStream<WebSocket>,
    who: SocketAddr,
    client_name: Arc<sync::RwLock<Option<String>>>
) {
    let who = Who { who, client_name };
    let mut interval = tokio::time::interval(Duration::from_secs(8));
    // Must make sure a pong is received before the next ping is sent out.
    let time = Duration::from_secs(5);
    loop {
        interval.tick().await;

        let ping = timeout(time, async {
            let mut socket = sender.lock().await;
            match socket.send(Message::Ping(vec![])).await {
                Ok(_) => trace!("[{who}] Sent Ping"),
                Err(_) => {
                    warn!("[{who}] Could not Ping");
                    match socket.close().await {
                        Ok(_) => warn!("[{who}] Closed socket"),
                        Err(_) => warn!("[{who}] Could not close socket, maybe already closed"),
                    }
                    return Err(());
                },
            };
            loop {
                match receiver.next().await {
                    Some(msg) => match msg {
                        Ok(Message::Pong(_)) => break Ok(()),
                        Ok(Message::Close(_)) => {
                            info!("{who} Received Close message");
                            break Err(());
                        },
                        Err(e) => {
                            error!("{who} Error receiving messages: {e:?}");
                            break Err(());
                        }
                        Ok(m) => warn!("{who} Received irrelevant message: {m:?}")
                    },
                    None => error!("{who} Error: receiver is empty")
                };
            }
        });
        match ping.await {
            Ok(Ok(_)) => trace!("[{who}] Pong received"),
            Ok(Err(_)) => {
                warn!("{who} Exiting heartbeat loop");
                return;
            },
            Err(_) => {
                warn!("{who} No Pong response before timeout, exiting heartbeat loop");
                return;
            },
        };
    };
}

#[derive(Clone)]
struct Who {
    who: SocketAddr,
    client_name: Arc<sync::RwLock<Option<String>>>
}

impl Display for Who {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}", self.who)?;
        match self.client_name.read() {
            Ok(read) =>
                match read.as_ref() {
                    Some(c) => write!(f, ", {c}")?,
                    None => (),
                },
            Err(e) =>
                error!("Can't get read handle: {}", e)
        }
        write!(f, "]")?;
        Ok(())
    }
}
