use std::{sync::{Mutex, Arc}, path::Path};

use axum::{extract::{State, Multipart}, Json, response::IntoResponse};
use axum_macros::debug_handler;
use hyper::{StatusCode, Request, Uri, Body};
use redis::{aio::Connection, Client, AsyncCommands, JsonAsyncCommands};
use regex::Regex;
use serde::{Serialize, Deserialize};
use tokio::{sync::Mutex as AsyncMutex, io::AsyncWriteExt, fs::File as TokioFile};
use tokio_util::bytes::Bytes;
use tower::ServiceExt;
use tower_http::services::ServeFile;
use tracing::{error_span, info_span, warn_span};
use uuid::Uuid;

use crate::AppState;

pub type Response<T> = Result<Json<T>, (StatusCode, Json<T>)>;

#[derive(Serialize)]
#[serde(tag = "type", content = "content")]
pub enum Payload {
    FilePaths(Vec<String>),
    Error {
        code: u8,
        message: String
    }
}

#[debug_handler]
pub async fn get_files(State(state): State<AppState>) -> Response<Payload> {
    Ok(Json(Payload::FilePaths(state.file_server.lock().await.get_paths().await)))
}

#[debug_handler]
pub async fn get_file(
    State(state): State<AppState>,
    uri: Uri
) -> impl IntoResponse {
    let file_server = state.file_server.lock().await;
    let path = file_server.get_file(uri.to_string()).await;

    match path {
        Some(p) => {
            let req = Request::builder().uri(uri.clone()).body(Body::empty()).unwrap();
            let f = ServeFile::new(format!("file_server/{p}"));
            match f.oneshot(req).await {
                Ok(res) => Ok(res.map(axum::body::boxed)),
                Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
            }
        },
        None => Err(StatusCode::NOT_FOUND),
    }
}

struct AddFiles {
    directory: String,
    files: Vec<(String, Bytes)>,
}

#[debug_handler]
pub async fn add_files(
    State(state): State<AppState>,
    mut multipart: Multipart
) -> Response<Payload> {
    let mut file_server = state.file_server.lock().await;
    let mut add_files = AddFiles {
        directory: String::new(),
        files: Vec::new()
    };

    while let Some(field) = multipart.next_field().await.unwrap() {

        if let Some(filename) = field.file_name() {
            let filename = filename.to_string();
            let bytes = field.bytes().await.unwrap();
            info_span!("Add file ", filename);
            add_files.files.push((filename, bytes));
        } else if let Some(name) = field.name() {
            // res.append(&mut vec!["WOHOO".to_string(), name.to_string(), field.text().await.unwrap()]);
            if name == "directory" {
                let directory = field.text().await
                    .unwrap_or(String::new())
                    .trim()
                    .trim_end_matches("/")
                    .to_string();
                info_span!("Got directory name", directory);
                add_files.directory = directory;
            }
            continue;
        } else {
            warn_span!("Unknown field", ?field);
        }
    }

    if add_files.directory.is_empty() {
        return Err((StatusCode::BAD_REQUEST, Json(Payload::Error {
            code: 2,
            message: format!("Directory cannot be empty")
        })))
    }

    let mut files = Vec::with_capacity(add_files.files.len());
    for (filename, bytes) in add_files.files {
        match file_server.add_file(format!("{}/{filename}", add_files.directory)).await {
            Ok(f) => files.push((f, bytes)),
            Err(message) => return Err((StatusCode::BAD_REQUEST, Json(Payload::Error { code: 3, message }))),
        }
    }

    file_server.write().await;

    for (f, bytes) in files {
        let mut file = TokioFile::create(format!("file_server/{}", f.file_server)).await.unwrap();
        file.write_all(&bytes).await.unwrap();
    }

    Ok(Json(Payload::FilePaths(vec![])))
}


#[derive(Serialize, Deserialize, Clone)]
pub struct File {
    name: String,
    file_server: String
}

#[derive(Clone)]
struct Directory {
    name: String,
    files: Arc<Mutex<Vec<File>>>,
    children: Arc<Mutex<Vec<Directory>>>,
}

#[derive(Deserialize, Serialize)]
struct DesDir {
    name: String,
    files: Vec<File>,
    children: Vec<Directory>,
}

impl<'de> Deserialize<'de> for Directory {

    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de> {
        let input = DesDir::deserialize(deserializer)?;
        Ok(Self {
            name: input.name,
            files: Arc::new(Mutex::new(input.files)),
            children: Arc::new(Mutex::new(input.children)),
        })
    }
}

impl Serialize for Directory {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer {
        DesDir::serialize(
            &DesDir {
                    name: self.name.clone(),
                    files: self.files.lock().unwrap().to_vec(),
                    children: self.children.lock().unwrap().to_vec()
                },
            serializer
        )
    }
}

pub struct FileServer {
    con: AsyncMutex<Connection>,
    root: Directory,
}

impl FileServer {
    pub async fn new() -> Self {
        let client = Client::open("redis://127.0.0.1:6379").unwrap();
        let con = client.get_async_connection().await.unwrap();
        let root = Directory {
            name: "".to_string(),
            files: Arc::new(Mutex::new(vec![])),
            children: Arc::new(Mutex::new(vec![]))
        };

        Self { con: AsyncMutex::new(con), root }
    }

    pub async fn get_paths(&self) -> Vec<String> {
        let mut con = self.con.lock().await;

        let _paths: Vec<String> = con.hkeys("files").await.unwrap();

        todo!()
    }

    /// Add file name to directory tree, and create folder if they don't already exists
    ///
    /// Does not call write to avoid writing when not all files are returning Ok()
    pub async fn add_file(&mut self, file_path: String) -> Result<File, String> {
        info_span!("Adding file with ", file_path);
        let file_path = file_path.trim();
        let re = Regex::new(r"^/[\w/_\-\.]+[\w]$").unwrap();
        if !re.is_match(file_path) {
            return Err("Illegal file name. Must only contain '_-./' special characters, start with root ('/') and end with a letter.".to_string())
        }

        let mut path = file_path
            .split('/')
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>();
        let file_name = path.pop().unwrap();

        let dir = self.traverse_to_dir(path).await;

        let mut files = dir.files.lock().unwrap();
        match files.binary_search_by_key(&file_name, |f| &f.name) {
            Ok(_) =>
                Err(format!("File {file_path} already exists")),
            Err(pos) => {
                files.insert(pos, File {
                    name: file_name.to_string(),
                    file_server: format!("{}.{}", Uuid::new_v4(), Path::new(file_name).extension().unwrap().to_str().unwrap()),
                    //TODO: Add things like path to file on disk (with uuid generated name)
                });

                Ok(files[pos].clone())
            },
        }
    }

    async fn get_file(&self, file_path: String) -> Option<String> {
        let mut path = file_path
            .split('/')
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>();
        let file_name = path.pop()?;

        let dir = self.traverse_to_dir(path).await;

        let files = dir.files.lock().unwrap();
        match files.binary_search_by_key(&file_name, |f| &f.name) {
            Ok(pos) =>
                Some(files[pos].file_server.clone()),
            Err(_) =>
                None,
        }
    }

    async fn traverse_to_dir(&self, path: Vec<&str>) -> Directory {
        let mut dir = self.root.clone();
        for p in path {
            let dir_c = dir.clone();
            let mut d = dir_c.children.lock().unwrap();
            let pos = match d.binary_search_by_key(&p, |d| &d.name) {
                // Update dir to dir and traverse down the tree
                Ok(pos) => pos,
                // Add Dir at sorted position in Vec if not present
                Err(pos) => {
                    d.insert(pos, Directory {
                        name: p.to_string(),
                        files: Arc::new(Mutex::new(vec![])),
                        children: Arc::new(Mutex::new(vec![])),
                    });
                    pos
                },
            };
            dir = d.get(pos).unwrap().clone();
        }
        dir
    }

    async fn write(&mut self) {
        let root_dir = &self.root.clone();
        if let Err(error) = self.con.lock().await.json_set::<_, _, _, String>("files", "$", &root_dir).await {
            error_span!("Redis Error", ?error);
            // error_span!("Logging current state instead", ?self.content);
        }
    }
}