use std::{fs::File, io::Read};

use axum::extract::Path;
use crypto::digest::Digest;
use maud::{html, Markup, PreEscaped, DOCTYPE};
use sha2::Sha256;
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

pub fn compute_hash(file_paths: Vec<&std::path::PathBuf>) -> String {
    let mut hasher = Sha256::new();
    for file_path in file_paths {
        let mut file = File::open(file_path).unwrap();

        let mut buffer = Vec::new();
        loop {
            let bytes_read = file.read_to_end(&mut buffer).unwrap();
            if bytes_read == 0 {
                break;
            }
            hasher.update(&buffer);
            buffer.clear(); // Clear the buffer for the next read
        }
    }

    base16ct::lower::encode_string(&hasher.finalize())
}
