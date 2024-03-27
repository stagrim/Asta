use casta_protocol::{uuid::Uuid, RequestPayload, ResponsePayload as SastaPayload};
use futures_util::{
    stream::{SplitSink, SplitStream},
    SinkExt, StreamExt,
};
use tokio::{net::TcpStream, time};
use tokio_tungstenite::{connect_async, tungstenite::Message, MaybeTlsStream, WebSocketStream};
use url::Url;

type Socket = WebSocketStream<MaybeTlsStream<TcpStream>>;

pub struct Sasta {
    address: String,
    port: String,
    uuid: Uuid,
    ws_sender: SplitSink<Socket, Message>,
    ws_receiver: SplitStream<Socket>,
}

impl Sasta {
    async fn connect(
        address: String,
        port: String,
        uuid: Uuid,
    ) -> (SplitSink<Socket, Message>, SplitStream<Socket>) {
        let wait_sec = 5;
        let mut interval = time::interval(time::Duration::from_secs(wait_sec));

        let mut socket = None;

        while socket.is_none() {
            interval.tick().await;
            let url = Url::parse(&format!("ws://{address}:{port}")).unwrap();
            socket = match connect_async(url.clone()).await {
                Ok(r) => Some(r.0),
                Err(_) => {
                    println!(
                        r#"[Error] Could not connect to "{url}", retrying in {wait_sec} seconds"#
                    );
                    None
                }
            };
        }

        let hello = RequestPayload::Hello { uuid, htmx: false };
        let (mut w, r) = socket.unwrap().split();

        w.send(Message::Text(serde_json::to_string(&hello).unwrap()))
            .await
            .expect("Could not write message");
        println!("Connected");

        (w, r)
    }

    pub async fn new(address: String, port: String, uuid: Uuid) -> Self {
        let (ws_sender, ws_receiver) =
            Self::connect(address.clone(), port.clone(), uuid.clone()).await;

        Sasta {
            address,
            port,
            uuid,
            ws_sender,
            ws_receiver,
        }
    }

    pub async fn reconnect(&mut self) {
        match self.ws_sender.close().await {
            Ok(_) => println!("Closed Sasta connection"),
            Err(e) => println!("Could not close Sasta connection: {e:?}"),
        }
        println!("Connection closed, attempting reconnect");
        let (s, r) =
            Self::connect(self.address.clone(), self.port.clone(), self.uuid.clone()).await;

        self.ws_sender = s;
        self.ws_receiver = r;
    }

    /// Reads incoming messages from Sasta. Responds to Ping with a Pong, and returns Text parsed as a SastaResponse. Returns None if not having received a message within specified duration,
    pub async fn read_message(&mut self) -> Option<SastaPayload> {
        let timeout = time::Duration::from_secs(20);
        loop {
            let res = match time::timeout(timeout, self.ws_receiver.next()).await {
                Ok(r) => r.expect("Got None"),
                Err(_) => {
                    println!(
                        "Has not received a message in {} seconds, server seems to be dead",
                        timeout.as_secs()
                    );
                    return None;
                }
            };

            let msg = match res {
                Ok(m) => m,
                Err(e) => {
                    println!("{e}");
                    return None;
                }
            };

            match msg {
                Message::Text(s) => {
                    // println!("{:#?}", s);
                    return serde_json::from_str(&s)
                        .expect(&format!("Cannot parse string: {:?}", s));
                }
                Message::Ping(_) => {
                    println!("[Ping]");
                }
                _ => (),
            };
        }
    }
}
