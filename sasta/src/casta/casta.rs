use axum::extract::Path;
use maud::{html, Markup, PreEscaped, DOCTYPE};
use uuid::Uuid;

pub async fn casta_index(Path(uuid): Path<Uuid>) -> Markup {
    html! {
        (DOCTYPE)
        html lang="en" {
            head {
                meta charset="UTF-8";
                meta name="viewport" content="width=device-width, initial-scale=1.0";
                link rel="stylesheet" href="/assets/style.css";
                script src="https://unpkg.com/htmx.org@1.9.2" {}
                script src="https://unpkg.com/htmx.org/dist/ext/ws.js" {}
                script { (PreEscaped("let uuid = \"")) (uuid) (PreEscaped("\"")) }
                script src="/assets/script.js" {}
            }
            body {
                img #disconnected src="/assets/disconnected.png";
                div #content hx-ext="ws" ws-connect="/ws";
            }
        }
    }
}
