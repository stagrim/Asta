use std::net::TcpStream;

use serde::Serialize;
use tokio::time;
use tokio_tungstenite::{tungstenite::{connect, Message, WebSocket, stream::MaybeTlsStream, self}};
use url::Url;

type Socket = WebSocket<MaybeTlsStream<TcpStream>>;

#[derive(Serialize)]
struct SastaHello {
    uuid: String,
    hostname: String
}

pub struct Sasta {
    address: String,
    port: String,
    uuid: String,
    hostname: String,
    socket: Socket,
}


impl Sasta {
    async fn connect(address: String, port: String, uuid: String, hostname: String) -> Socket {
        let wait_sec = 5;
        let mut interval = time::interval(time::Duration::from_secs(wait_sec));

        let mut socket: Option<Socket> = None;

        while socket.is_none() {
            interval.tick().await;
            let url = Url::parse(&format!("ws://{address}:{port}")).unwrap();
            socket = match connect(url.clone()) {
                Ok(r) => Some(r.0),
                Err(_) => {
                    println!(r#"[Error] could not connect to "{url}", retrying in {wait_sec} seconds"#);
                    None
                },
            };
        }

        let hello = SastaHello {
            uuid,
            hostname,
        };
        socket.as_mut().unwrap().write_message(Message::Text(serde_json::to_string(&hello).unwrap()))
            .expect("Could not write message");
        println!("Connected");

        socket.unwrap()
    }

    pub async fn new(address: String, port: String, uuid: String, hostname: String) -> Self {
        let socket = Self::connect(address.clone(), port.clone(), uuid.clone(), hostname.clone()).await;

        Sasta { address, port, uuid, hostname, socket }
    }

    pub async fn reconnect(&mut self) {
        self.socket.close(None).unwrap();
        println!("Connection closed, attempting reconnect");
        let socket = Self::connect(self.address.clone(), self.port.clone(), self.uuid.clone(), self.hostname.clone());

        self.socket = socket.await;
    }

    //TODO: respond with structs instead and handle them in main
    pub async fn read_message(&mut self) -> String {
        loop {            
            match self.socket.read_message() {
                Ok(msg) => {
                    match msg {
                        tungstenite::Message::Text(s) => { 
                            return s;
                        }
                        tungstenite::Message::Ping(_) => println!("Ping"),
                        _ => (),
                    };
                    // println!("{}", msg);
                    // let parsed: serde_json::Value = serde_json::from_str(&msg).expect("Can't parse to JSON");
                    // println!("{:?}", parsed["result"]);
                },
                Err(e) => {
                    println!("{e}");
                    self.reconnect().await;
                }
            }
        }
    }
}
