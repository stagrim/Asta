use std::{
    fmt::Display,
    net::SocketAddr,
    sync::{self, Arc},
    time::Duration,
};

use axum::{
    body::Bytes,
    extract::ws::{Message, WebSocket},
};
use casta_protocol::{DisplayPayload, RequestPayload, ResponsePayload, WebsitePayload};
use futures_util::{
    stream::{SplitSink, SplitStream},
    SinkExt, StreamExt,
};
use maud::html;
use tokio::{
    sync::Mutex,
    time::{sleep_until, timeout, Instant},
};
use tracing::{error, info, trace, warn};

use crate::store::store::{Change, ImageData, PlaylistItem, Store, TextData, WebsiteData};

trait IntoHtmx {
    fn into_htmx(&self) -> String;
}

impl IntoHtmx for DisplayPayload {
    fn into_htmx(&self) -> String {
        html! { div hx-swap-oob="innerHTML:#content" {
        @match self {
            DisplayPayload::Website(data) => iframe frameborder="0" allow="autoplay; encrypted-media" src=(data.content) allowfullscreen;,
            DisplayPayload::Text(data) => {
                div #text { (data.content) }
            }
            DisplayPayload::Image(data) => img src=(data.content);,
        }
    }}
        .into_string()
    }
}

/// Websocket connection to the client following the casta protocol
///
/// htmx_hash will be sent to the client using giving the htmx option
/// and should reflect any changes to the htmx client hosted on the server.
/// The fetched client is expected to refresh to get the latest version if the
/// hash sent in the welcome response does not match the previously stored hash.
pub async fn client_connection(
    socket: WebSocket,
    who: SocketAddr,
    store: Arc<Store>,
    htmx_hash: String,
) {
    let (client_send, mut client_receive) = socket.split();
    let client_send = Arc::new(Mutex::new(client_send));

    // Wait for a hello response from connected client to get its UUID
    let (client_uuid, htmx) = loop {
        match client_receive.next().await {
            Some(Ok(Message::Text(msg))) => {
                match serde_json::from_str::<RequestPayload>(&msg) {
                    Ok(RequestPayload::Hello { uuid, htmx }) => break (uuid, htmx),
                    _ => error!("[{who}] {msg:?} was not a HelloRequest"),
                };
            }
            err => {
                error!("[{who}] Received '{err:?}' instead of HelloRequest, exiting");
                return;
            }
        }
    };

    let client_name = Arc::new(sync::RwLock::new(None));

    info!("[{who}] Connected with provided Uuid '{client_uuid}' and htmx set to '{htmx}'");

    let mut heartbeat_handle = tokio::spawn(heartbeat(
        client_send.clone(),
        client_receive,
        who,
        client_name.clone(),
    ));

    let mut client_handle = tokio::spawn(async move {
        let mut rx = store.receiver();
        loop {
            let display_option = store
                .read()
                .await
                .displays
                .get(&client_uuid)
                .and_then(|d| Some(d.clone()));
            match display_option {
                Some(d) => {
                    let mut w = client_name.write().unwrap();
                    *w = Some(d.name);
                    break;
                }
                None => {
                    warn!(
                        "[{who}] No screen defined with UUID {:?}, waiting for Display update",
                        client_uuid
                    );
                    let msg = if htmx {
                        Message::Text(
                            DisplayPayload::Text(WebsitePayload {
                                content: format!(
                                    "Pending connection to Sasta with uuid {}",
                                    &client_uuid
                                ),
                            })
                            .into_htmx()
                            .into(),
                        )
                    } else {
                        Message::Text(
                            serde_json::to_string(&ResponsePayload::Pending(true))
                                .unwrap()
                                .into(),
                        )
                    };

                    if let Err(e) = client_send.lock().await.send(msg).await {
                        error!("[{who}] Could not send Pending message because '{e:?}', exiting");
                        return;
                    };

                    // Wait until display is updated change, then try again
                    loop {
                        match rx.recv().await {
                            Ok(msg) => match msg {
                                Change::Display(m) if m.contains(&client_uuid) => break,
                                _ => (),
                            },
                            Err(err) => {
                                error!("[{who}] received error '{err:?}', exiting");
                                return;
                            }
                        };
                        trace!("loop 2");
                    }
                    continue;
                }
            }
        }

        let client_name = client_name
            .read()
            .unwrap()
            .clone()
            .unwrap_or("Got a None".to_string());

        let msg = Message::Text(
            serde_json::to_string(&ResponsePayload::Welcome {
                name: client_name.clone(),
                htmx_hash: htmx.then(|| htmx_hash),
            })
            .unwrap()
            .into(),
        );

        if let Err(e) = client_send.lock().await.send(msg).await {
            error!(
                "[{who} ({client_name})] Could not send Welcome message because '{e:?}', exiting"
            );
            return;
        };

        info!("[{who}] was given the name {client_name}");

        // outer loop collects the PlaylistItems(s) before entering the repeating send loop
        'outer_send_loop: loop {
            let (schedule_uuid, playlist_uuid) =
                store.get_display_uuids(&client_uuid).await.unwrap();
            let mut playlist = match store.get_display_playlist_items(&client_uuid).await {
                Some(p) => p,
                None => {
                    error!("[{who} ({client_name})] Error: Display playlist could not be found");
                    return;
                }
            };

            // If playlist is empty, add text stating such to display loop
            if playlist.is_empty() {
                playlist.push(PlaylistItem::Text {
                    id: "pending".into(),
                    settings: TextData {
                        text: "No Playlist added".into(),
                        duration: 0,
                    },
                });
            }

            for item in playlist.into_iter().cycle() {
                let sleep_duration;
                let payload = match item {
                    PlaylistItem::Website {
                        id: name,
                        settings: WebsiteData { url, duration },
                    } => {
                        info!("[{who} ({client_name})] Sending Website '{name}'");
                        sleep_duration = duration;
                        DisplayPayload::Website(WebsitePayload { content: url })
                    }
                    PlaylistItem::Text {
                        id: name,
                        settings: TextData { text, duration },
                    } => {
                        info!("[{who} ({client_name})] Sending Text '{name}'");
                        sleep_duration = duration;
                        DisplayPayload::Text(WebsitePayload { content: text })
                    }
                    PlaylistItem::Image {
                        id: name,
                        settings: ImageData { src, duration },
                    } => {
                        info!("[{who} ({client_name})] Sending Image '{name}'");
                        sleep_duration = duration;
                        DisplayPayload::Image(WebsitePayload { content: src })
                    }
                    PlaylistItem::BackgroundAudio { .. } => todo!(),
                };

                let msg = if htmx {
                    Message::Text(payload.into_htmx().into())
                } else {
                    Message::Text(
                        serde_json::to_string(&ResponsePayload::Display(payload))
                            .unwrap()
                            .into(),
                    )
                };
                if let Err(e) = client_send.lock().await.send(msg).await {
                    error!("[{who} ({client_name})] Could not send playlist message because '{e:?}', exiting");
                    return;
                };

                let now = Instant::now();
                // Sleep for an infinite time if duration of the PlaylistItem is zero.
                // Maybe not "infinite" in a literal sense, but at least for some billion years,
                // which I would consider good enough.
                // A bug that needs to be fixed is that this will stop working in about hundred billion years
                // and the thread will panic with a overflow error. Note that this will happen for both the
                // debug and release builds. Truly no one is safe from the catastrophe that will occur...
                let sleep = now
                    + Duration::from_secs(if sleep_duration == 0 {
                        u64::MAX / 10 as u64
                    } else {
                        sleep_duration
                    });

                loop {
                    info!(
                        "[{who} ({client_name})] Sleeping for {} seconds",
                        (sleep - now).as_secs()
                    );
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
                                        Change::Schedule(s) if schedule_uuid.is_some_and(|u| s.contains(&u)) => {
                                            info!("[{who} ({client_name})] Schedule {} has changed, restarting send loop", schedule_uuid.unwrap_or_default());
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
    client_name: Arc<sync::RwLock<Option<String>>>,
) {
    let who = Who { who, client_name };
    let mut interval = tokio::time::interval(Duration::from_secs(8));
    // Must make sure a pong is received before the next ping is sent out.
    let time = Duration::from_secs(5);
    loop {
        interval.tick().await;

        let ping = timeout(time, async {
            let mut socket = sender.lock().await;
            match socket.send(Message::Ping(Bytes::new())).await {
                Ok(_) => trace!("[{who}] Sent Ping"),
                Err(_) => {
                    warn!("[{who}] Could not Ping");
                    match socket.close().await {
                        Ok(_) => warn!("[{who}] Closed socket"),
                        Err(_) => warn!("[{who}] Could not close socket, maybe already closed"),
                    }
                    return Err(());
                }
            };
            loop {
                match receiver.next().await {
                    Some(msg) => match msg {
                        Ok(Message::Pong(_)) => break Ok(()),
                        Ok(Message::Close(_)) => {
                            info!("{who} Received Close message");
                            break Err(());
                        }
                        Err(e) => {
                            error!("{who} Error receiving messages: {e:?}");
                            break Err(());
                        }
                        Ok(m) => warn!("{who} Received irrelevant message: {m:?}"),
                    },
                    None => error!("{who} Error: receiver is empty"),
                };
            }
        });
        match ping.await {
            Ok(Ok(_)) => trace!("[{who}] Pong received"),
            Ok(Err(_)) => {
                warn!("{who} Exiting heartbeat loop");
                return;
            }
            Err(_) => {
                warn!("{who} No Pong response before timeout, exiting heartbeat loop");
                return;
            }
        };
    }
}

#[derive(Clone)]
struct Who {
    who: SocketAddr,
    client_name: Arc<sync::RwLock<Option<String>>>,
}

impl Display for Who {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}", self.who)?;
        match self.client_name.read() {
            Ok(read) => match read.as_ref() {
                Some(c) => write!(f, ", {c}")?,
                None => (),
            },
            Err(e) => error!("Can't get read handle: {}", e),
        }
        write!(f, "]")?;
        Ok(())
    }
}
