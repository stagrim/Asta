
use std::{net::SocketAddr, sync::Arc, time::Duration};

use axum::extract::ws::{Message, WebSocket};
use futures_util::{SinkExt, stream::{SplitSink, SplitStream}, StreamExt};
use serde::{Deserialize, Serialize};
use tokio::{sync::Mutex, time::timeout};

use crate::store::store::{PlaylistItem, Store};

#[derive(Deserialize, Clone)]
struct HelloRequest {
    uuid: String,
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
    let hello: HelloRequest = loop {
        if let Some(Ok(Message::Text(msg))) = client_receive.next().await {
            match serde_json::from_str(&msg) {
                Ok(msg) => break msg,
                Err(_) => println!("{msg:?} was not a HelloRequest")
            };
        }
    };

    let mut heart_beat_handle = tokio::spawn(heart_beat(client_send.clone(), client_receive));

    let mut client_handle = tokio::spawn(async move {
        
        let mut rx = store.receiver();
        let display = loop {
            let display_option = store.read().await.displays.get(&hello.uuid).and_then(|d| Some(d.clone()));
            match display_option {
                Some(d) => break d,
                None => {
                    println!("No screen defined with UUID {:?}", hello.uuid);
                    let msg = Message::Text(serde_json::to_string(&Payload::Pending(true)).unwrap());
                    client_send.lock().await.send(msg).await.unwrap();

                    // Wait until content change, then try again 
                    rx.changed().await.unwrap();
                    continue;
                },
            }
        };
    
        let msg = Message::Text(serde_json::to_string(&Payload::Name(display.name)).unwrap());
        client_send.lock().await.send(msg).await.unwrap();

        // outer loop collects the PlaylistItems(s) before entering the repeating send loop
        'outer_send_loop: loop {
            let playlist = match store.get_display_playlists(&hello.uuid).await {
                Some(p) => p,
                None => {
                    println!("Error: Display playlist could not be found");
                    return
                },
            };
        
            for item in playlist.iter().cycle() {
                let sleep;
                let payload = match item {
                    PlaylistItem::Website { name, settings } => {
                        println!("Sending {name:?} website to {:?}", hello.hostname);
                        sleep = settings.duration;
                        DisplayPayload::Website { data: WebsitePayload { content: settings.url.clone() } }
                    },
                    PlaylistItem::Text { name, settings } => {
                        println!("Sending {name:?} text to {:?}", hello.hostname);
                        sleep = settings.duration;
                        DisplayPayload::Text { data: WebsitePayload { content: settings.text.clone() } }
                    },
                    PlaylistItem::Image { name, settings } => {
                        println!("Sending {name:?} image to {:?}", hello.hostname);
                        sleep = settings.duration;
                        DisplayPayload::Image { data: WebsitePayload { content: settings.src.clone() } }
                    },
                    
                };
    
                let msg = Message::Text(serde_json::to_string(&Payload::Display(payload)).unwrap());
                client_send.lock().await.send(msg).await.unwrap();
                
                if let Ok(notification) = timeout(Duration::from_secs(sleep), rx.changed()).await {
                    match notification {
                        Ok(_) => {
                            println!("Message received, restarting send loop");
                            continue 'outer_send_loop;
                        },
                        Err(e) => {
                            println!("Exit thread due to error: {e}");
                            return;
                        },
                    }
                }
            }
        }
    });

    tokio::select! {
        _ = &mut heart_beat_handle  => client_handle.abort(),
        _ = &mut client_handle      => heart_beat_handle.abort(),
    };

    println!("Done with {who}!");
}


async fn heart_beat(sender: Arc<Mutex<SplitSink<WebSocket, Message>>>, mut receiver: SplitStream<WebSocket>) {
    let mut interval = tokio::time::interval(Duration::from_secs(3));
    // Must make sure a pong is received before the next ping is sent out.
    let time = Duration::from_secs(3);
    loop {
        interval.tick().await;
        
        let ping = timeout(time, async {
            let mut socket = sender.lock().await;
            match socket.send(Message::Ping(vec![])).await {
                Ok(_) => println!("Sent Ping"),
                Err(_) => {
                    println!("Could not ping, fuck");
                    match socket.close().await {
                        Ok(_) => println!("hej"),
                        Err(_) => println!("nej"),
                    }
                    return Err(());
                },
            };
            loop {
                if let Some(msg) = receiver.next().await {
                    match msg {
                        Ok(Message::Pong(_)) => break,
                        Err(e) => {
                            println!("Error receiving messages: {e:?}");
                            return Err(());
                        }
                        _ => ()
                    }
                };
            };
            Ok(())
        });
        match ping.await {
            Ok(Ok(_)) => println!("Pong received"),
            _ => {
                println!("No Pong response before timeout, disconnecting from client");
                return;
            },
        };
    };
}