
use std::{net::SocketAddr, sync::Arc, time::Duration};

use axum::extract::ws::{Message, WebSocket};
use futures_util::{SinkExt, stream::{SplitSink, SplitStream}, StreamExt};
use serde::{Deserialize, Serialize};
use tokio::{sync::Mutex, time::{timeout, Instant, sleep_until}};
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
                Err(_) => println!("[{who}] {msg:?} was not a HelloRequest")
            };
        }
    };

    println!("[{who}] Connected with provided Uuid '{client_uuid}' and hostname '{client_hostname}'");

    let mut heartbeat_handle = tokio::spawn(heartbeat(client_send.clone(), client_receive, who));

    let mut client_handle = tokio::spawn(async move {
        
        let mut rx = store.receiver();
        let client_name = loop {
            let display_option = store.read().await.displays.get(&client_uuid).and_then(|d| Some(d.clone()));
            match display_option {
                Some(d) => break d.name,
                None => {
                    println!("[{who}] No screen defined with UUID {:?}, waiting for Display update", client_uuid);
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
    
        let msg = Message::Text(serde_json::to_string(&Payload::Name(client_name.clone())).unwrap());
        client_send.lock().await.send(msg).await.unwrap();

        println!("[{who}] was given the name {client_name}");

        // outer loop collects the PlaylistItems(s) before entering the repeating send loop
        'outer_send_loop: loop {
            let (schedule_uuid, playlist_uuid) = store.get_display_uuids(&client_uuid).await.unwrap();
            let playlist = match store.get_display_playlists(&client_uuid).await {
                Some(p) => p,
                None => {
                    println!("[{who} ({client_name})] Error: Display playlist could not be found");
                    return
                },
            };
        
            for item in playlist.into_iter().cycle() {
                let sleep_duration;
                let payload = match item {
                    PlaylistItem::Website { name, settings: WebsiteData { url, duration } } => {
                        println!("[{who} ({client_name})] Sending Website '{name}' with url '{}'", url);
                        sleep_duration = duration;
                        DisplayPayload::Website { data: WebsitePayload { content: url } }
                    },
                    PlaylistItem::Text { name, settings: TextData { text, duration } } => {
                        println!("[{who} ({client_name})] Sending Text '{name}' with text '{text}'");
                        sleep_duration = duration;
                        DisplayPayload::Text { data: WebsitePayload { content: text } }
                    },
                    PlaylistItem::Image { name, settings: ImageData { src, duration } } => {
                        println!("[{who} ({client_name})] Sending Image '{name}' with src '{src}'");
                        sleep_duration = duration;
                        DisplayPayload::Image { data: WebsitePayload { content: src } }
                    },
                    
                };
    
                let msg = Message::Text(serde_json::to_string(&Payload::Display(payload)).unwrap());
                client_send.lock().await.send(msg).await.unwrap();
                
                let sleep = Instant::now() + Duration::from_secs(sleep_duration);
                
                loop {
                    tokio::select! {
                        _ = sleep_until(sleep) => break,
                        notification = rx.recv() => {
                            match notification {
                                Ok(c) => {
                                    match c {
                                        Change::Display(d) if d.contains(&client_uuid) => {
                                            println!("[{who} ({client_name})] Display {} has changed, restarting send loop", client_uuid);
                                            continue 'outer_send_loop
                                        },
                                        Change::Playlist(p) if p.contains(&playlist_uuid)  => {
                                            println!("[{who} ({client_name})] Playlist {} has changed, restarting send loop", playlist_uuid);
                                            continue 'outer_send_loop
                                        },
                                        Change::Schedule(s) if s.contains(&schedule_uuid) => {
                                            println!("[{who} ({client_name})] Schedule {} has changed, restarting send loop", schedule_uuid);
                                            continue 'outer_send_loop
                                        },
                                        _ => {
                                            println!("[{who} ({client_name})] Message received but no Change relate to current client, skipping")
                                        }
                                    }
                                },
                                Err(e) => {
                                    println!("[{who} ({client_name})] Exit thread due to error: {e}");
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
        _ = &mut client_handle      => heartbeat_handle.abort(),
    };

    println!("[{who}] Disconnected from client!");
}


async fn heartbeat(sender: Arc<Mutex<SplitSink<WebSocket, Message>>>, mut receiver: SplitStream<WebSocket>, who: SocketAddr) {
    let mut interval = tokio::time::interval(Duration::from_secs(8));
    // Must make sure a pong is received before the next ping is sent out.
    let time = Duration::from_secs(5);
    loop {
        interval.tick().await;
        
        let ping = timeout(time, async {
            let mut socket = sender.lock().await;
            match socket.send(Message::Ping(vec![])).await {
                Ok(_) => println!("[{who}] Sent Ping"),
                Err(_) => {
                    println!("[{who}] Could not Ping");
                    match socket.close().await {
                        Ok(_) => println!("[{who}] Closed socket"),
                        Err(_) => println!("[{who}] Could not close socket, maybe already closed"),
                    }
                    return Err(());
                },
            };
            loop {
                match receiver.next().await {
                    Some(msg) => match msg {
                        Ok(Message::Pong(_)) => break Ok(()),
                        Ok(Message::Close(_)) => {
                            println!("[{who}] Received Close message");
                            break Err(());
                        },
                        Err(e) => {
                            println!("[{who}] Error receiving messages: {e:?}");
                            break Err(());
                        }
                        Ok(m) => println!("[{who}] Received irrelevant message: {m:?}")
                    },
                    None => println!("[{who}] Error: receiver is empty")
                };
            }
        });
        match ping.await {
            Ok(Ok(_)) => println!("[{who}] Pong received"),
            Ok(Err(_)) => {
                println!("[{who}] Exiting heartbeat loop");
                return;
            },
            Err(_) => {
                println!("[{who}] No Pong response before timeout, exiting heartbeat loop");
                return;
            },
        };
    };
}