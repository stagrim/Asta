use std::{cmp::Ordering, fs, path::PathBuf};

#[cfg(not(debug_assertions))]
use std::{fs::File, io::Read};

use axum::extract::Path;
#[cfg(not(debug_assertions))]
use crypto::digest::Digest;
use lightningcss::{
    printer::PrinterOptions,
    stylesheet::{MinifyOptions, ParserOptions, StyleSheet},
};
use maud::{html, Markup, PreEscaped, DOCTYPE};
use minify_js::{Session, TopLevelMode};
#[cfg(not(debug_assertions))]
use sha2::Sha256;
use tracing::debug;
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

/// Check if first path is older (greater) then second path or not
fn cmp_modified(a: &PathBuf, b: &PathBuf) -> Ordering {
    let get_modified = |x: &PathBuf| x.metadata().and_then(|m| m.modified());
    match (get_modified(a), get_modified(b)) {
        (Ok(t1), Ok(t2)) => t1.cmp(&t2),
        _ => Ordering::Greater,
    }
}

pub fn minify() {
    let js_file_path = PathBuf::from("./assets/script.js");
    let js_file_min_path = PathBuf::from("./assets/script.min.js");

    if cmp_modified(&js_file_path, &js_file_min_path).is_gt() {
        let js_file = fs::read(js_file_path).unwrap();
        let session = Session::new();
        let mut js_min = Vec::new();
        minify_js::minify(&session, TopLevelMode::Global, &js_file, &mut js_min).unwrap();
        fs::write(js_file_min_path, js_min).unwrap();
    } else {
        debug!("min js file is newer, skipping");
    }

    let css_file_path = PathBuf::from("./assets/style.css");
    let css_file_min_path = PathBuf::from("./assets/style.min.css");

    if cmp_modified(&css_file_path, &css_file_min_path).is_gt() {
        let css_file = fs::read_to_string(css_file_path).unwrap();
        let mut stylesheet = StyleSheet::parse(&css_file, ParserOptions::default()).unwrap();
        stylesheet.minify(MinifyOptions::default()).unwrap();
        let css_min = stylesheet.to_css(PrinterOptions::default()).unwrap();
        fs::write(css_file_min_path, css_min.code).unwrap();
    } else {
        debug!("min css file is newer, skipping");
    }
}

pub fn compute_hash() -> String {
    #[cfg(not(debug_assertions))]
    {
        let file_paths = vec![
            std::env::current_exe().unwrap(),
            PathBuf::from("./assets/style.min.css"),
            PathBuf::from("./assets/script.min.js"),
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
    #[cfg(debug_assertions)]
    String::from("Server running in Debug Mode")
}
