use futures_util::{StreamExt, stream::{SplitSink, SplitStream}, SinkExt,};
use serde::{Serialize, Deserialize};
use tokio::{time, net::TcpStream};
use tokio_tungstenite::{tungstenite::Message, connect_async, WebSocketStream, MaybeTlsStream};
use url::Url;

type Socket = WebSocketStream<MaybeTlsStream<TcpStream>>;

//TODO: handle rejection from not sending a known uuid and display to uuid on the screen
#[derive(Deserialize, Debug)]
pub enum SastaResponse {
    #[serde(rename(deserialize = "display"))]
    Display(DisplayData),
    #[serde(rename(deserialize = "name"))]
    Name(String),
    #[serde(rename(deserialize = "pending"))]
    Pending(bool)
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(tag = "type")]
pub enum DisplayData {
    #[serde(rename(deserialize = "WEBSITE"))]
    Website { data: WebsiteData },
    #[serde(rename(deserialize = "TEXT"))]
    Text { data: WebsiteData },
    #[serde(rename(deserialize = "IMAGE"))]
    Image { data: WebsiteData }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct WebsiteData {
    pub content: String
}

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
    ws_sender: SplitSink<Socket, Message>,
    ws_receiver: SplitStream<Socket>,
}


impl Sasta {
    async fn connect(address: String, port: String, uuid: String, hostname: String) -> (SplitSink<Socket, Message>, SplitStream<Socket>) {
        let wait_sec = 5;
        let mut interval = time::interval(time::Duration::from_secs(wait_sec));

        let mut socket = None;

        while socket.is_none() {
            interval.tick().await;
            let url = Url::parse(&format!("ws://{address}:{port}")).unwrap();
            socket = match connect_async(url.clone()).await {
                Ok(r) => Some(r.0),
                Err(_) => {
                    println!(r#"[Error] Could not connect to "{url}", retrying in {wait_sec} seconds"#);
                    None
                },
            };
        }

        let hello = SastaHello {
            uuid,
            hostname,
        };
        let (mut w, r) = socket.unwrap().split();

        w.send(Message::Text(serde_json::to_string(&hello).unwrap())).await
            .expect("Could not write message");
        println!("Connected");

        (w, r)
    }

    pub async fn new(address: String, port: String, uuid: String, hostname: String) -> Self {
        let (ws_sender, ws_receiver) = Self::connect(address.clone(), port.clone(), uuid.clone(), hostname.clone()).await;

        Sasta { address, port, uuid, hostname, ws_sender, ws_receiver }
    }

    pub async fn reconnect(&mut self) {
        match self.ws_sender.close().await {
            Ok(_) => println!("Closed Sasta connection"),
            Err(e) => println!("Could not close Sasta connection: {e:?}"),
        }
        println!("Connection closed, attempting reconnect");
        let (s, r) = Self::connect(self.address.clone(), self.port.clone(), self.uuid.clone(), self.hostname.clone()).await;

        self.ws_sender = s;
        self.ws_receiver = r;
    }

    /// Reads incoming messages from Sasta. Responds to Ping with a Pong, and returns Text parsed as a SastaResponse. Returns None if not having received a message within specified duration, 
    pub async fn read_message(&mut self) -> Option<SastaResponse> {
        let timeout = time::Duration::from_secs(20);
        loop {
            let res = match time::timeout(timeout, self.ws_receiver.next()).await {
                Ok(r) => r.expect("Got None"),
                Err(_) => {
                    println!("Has not received a message in {} seconds, server seems to be dead", timeout.as_secs());
                    return None
                },
            };
            
            let msg = match res {
                Ok(m) => m,
                Err(e) => {
                    println!("{e}");
                    return None
                }
            };

            match msg {
                Message::Text(s) => {
                    // println!("{:#?}", s);
                    return serde_json::from_str(&s)
                        .expect(&format!("Cannot parse string: {:?}", s))
                },
                Message::Ping(_) => {
                    println!("[Ping]");
                }
                _ => (),
            };
        }
    }
}
