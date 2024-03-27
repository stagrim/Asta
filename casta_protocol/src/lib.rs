use serde::{Deserialize, Serialize};
pub use uuid;

#[derive(Deserialize, Serialize, Debug)]
#[serde(tag = "type", content = "data")]
pub enum ResponsePayload {
    Display(DisplayPayload),
    Welcome {
        name: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        htmx_hash: Option<String>,
    },
    Pending(bool),
}

#[derive(Deserialize, Serialize, Debug)]
#[serde(tag = "type", content = "data")]
pub enum RequestPayload {
    Hello {
        uuid: uuid::Uuid,
        #[serde(default)]
        htmx: bool,
    },
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(tag = "type", content = "data")]
pub enum DisplayPayload {
    Website(WebsitePayload),
    Text(WebsitePayload),
    Image(WebsitePayload),
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct WebsitePayload {
    pub content: String,
}
