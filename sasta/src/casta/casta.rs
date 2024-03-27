use std::{
    fs::{self, File},
    io::Read,
    path::PathBuf,
};

use axum::extract::Path;
use crypto::digest::Digest;
use lightningcss::{
    printer::PrinterOptions,
    stylesheet::{MinifyOptions, ParserOptions, StyleSheet},
};
use maud::{html, Markup, PreEscaped, DOCTYPE};
use minify_js::{Session, TopLevelMode};
use sha2::Sha256;
use tracing::info;
use uuid::Uuid;

pub async fn casta_index(Path(uuid): Path<Uuid>) -> Markup {
    html! {
        (DOCTYPE)
        html lang="en" {
            head {
                meta charset="UTF-8";
                meta name="viewport" content="width=device-width, initial-scale=1.0";
                link rel="stylesheet" href="/assets/style.min.css";
                script src="https://unpkg.com/htmx.org@1.9.2" {}
                script src="https://unpkg.com/htmx.org/dist/ext/ws.js" {}
                script { (PreEscaped("let uuid=\"")) (uuid) (PreEscaped("\"")) }
                script src="/assets/script.min.js" {}
            }
            body {
                img #disconnected src="/assets/disconnected.png";
                div #content hx-ext="ws" ws-connect="/ws";
            }
        }
    }
}

pub fn compute_hash() -> String {
    let file_paths = vec![
        std::env::current_exe().unwrap(),
        PathBuf::from("./assets/style.css"),
        PathBuf::from("./assets/script.js"),
    ];

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

pub fn minify() {
    let js_file = fs::read(PathBuf::from("./assets/script.js")).unwrap();
    let session = Session::new();
    let mut js_min = Vec::new();
    minify_js::minify(&session, TopLevelMode::Global, &js_file, &mut js_min).unwrap();
    fs::write("./assets/script.min.js", js_min).unwrap();

    let css_file = fs::read_to_string(PathBuf::from("./assets/style.css")).unwrap();
    let mut stylesheet = StyleSheet::parse(&css_file, ParserOptions::default()).unwrap();
    stylesheet.minify(MinifyOptions::default()).unwrap();
    let css_min = stylesheet.to_css(PrinterOptions::default()).unwrap();
    fs::write("./assets/style.min.css", css_min.code).unwrap();

    info!("JS and CSS minified");
}
