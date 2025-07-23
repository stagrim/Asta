use std::{
    collections::{LinkedList, VecDeque},
    path::Path,
    sync::{Arc, Mutex},
};

use axum::{
    body::Body,
    extract::{Multipart, State},
    response::IntoResponse,
    Json,
};
use axum_macros::debug_handler;
use chrono::{DateTime, Local};
use hyper::{Request, StatusCode, Uri};
use redis::{aio::MultiplexedConnection, Client, JsonAsyncCommands};
use regex::Regex;
use serde::{Deserialize, Serialize};
use tokio::{
    fs::{self, File as TokioFile},
    io::AsyncWriteExt,
    sync::Mutex as AsyncMutex,
};
use tokio_util::bytes::Bytes;
use tower::ServiceExt;
use tower_http::services::ServeFile;
use tracing::{error_span, info_span, warn, warn_span};
use ts_rs::TS;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::AppState;

pub type Response<T> = Result<Json<T>, (StatusCode, Json<T>)>;

#[derive(Serialize, ToSchema, TS)]
#[serde(tag = "type", content = "content")]
#[ts(export, export_to = "api_bindings/files/")]
pub enum Payload {
    FilePaths(ListView),
    Error { code: u8, message: String },
}

#[derive(Serialize, Debug, ToSchema, TS)]
#[ts(export, export_to = "api_bindings/files/")]
pub struct TreeView {
    content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// children is Some if content is a dir, None if content is a file
    children: Option<Vec<TreeView>>,
}

impl From<&File> for TreeView {
    fn from(value: &File) -> Self {
        TreeView {
            content: value.name.clone(),
            children: None,
        }
    }
}

impl From<&Directory> for TreeView {
    fn from(value: &Directory) -> Self {
        let mut children = Vec::new();
        children.append(
            &mut value
                .children
                .lock()
                .unwrap()
                .iter()
                .map(|f| f.into())
                .collect::<Vec<TreeView>>(),
        );
        children.append(
            &mut value
                .files
                .lock()
                .unwrap()
                .iter()
                .map(|f| f.into())
                .collect::<Vec<TreeView>>(),
        );
        TreeView {
            content: value.name.clone(),
            children: Some(children),
        }
    }
}

#[derive(Serialize, Debug, ToSchema, TS)]
#[ts(export, export_to = "api_bindings/files/")]
pub struct ListView(Vec<ListViewItem>);

#[derive(Serialize, Deserialize, Debug, ToSchema, TS)]
#[ts(export, export_to = "api_bindings/files/")]
pub struct ListViewItem {
    id: String,
    size: usize,
    date: String,
    r#type: Type,
}

#[derive(Serialize, Deserialize, Debug, ToSchema, TS)]
enum Type {
    #[serde(rename = "folder")]
    Directory,
    #[serde(rename = "file")]
    File,
}

impl From<&Directory> for ListView {
    fn from(value: &Directory) -> Self {
        let mut children = Vec::new();
        let mut visit_dirs = LinkedList::new();
        visit_dirs.push_back((value.children.clone(), value.files.clone()));
        while let Some((dirs, files)) = visit_dirs.pop_front() {
            let child_mutex = dirs.lock().unwrap();
            children.append(
                &mut child_mutex
                    .iter()
                    .inspect(|d| visit_dirs.push_back((d.children.clone(), d.files.clone())))
                    .map(|f| f.into())
                    .collect::<Vec<ListViewItem>>(),
            );
            children.append(
                &mut files
                    .lock()
                    .unwrap()
                    .iter()
                    .map(|f| f.into())
                    .collect::<Vec<ListViewItem>>(),
            );
        }
        ListView(children)
    }
}

impl From<&File> for ListViewItem {
    fn from(value: &File) -> Self {
        ListViewItem {
            id: value.path.clone(),
            size: value.size,
            date: value.date.to_rfc3339(),
            r#type: Type::File,
        }
    }
}
impl From<&Directory> for ListViewItem {
    fn from(value: &Directory) -> Self {
        ListViewItem {
            id: value.path.clone(),
            size: 0,
            date: "1996-12-19T16:39:57-08:00".to_string(),
            r#type: Type::Directory,
        }
    }
}

#[debug_handler]
pub async fn get_all_paths(State(state): State<AppState>) -> Response<Payload> {
    let files = state.file_server.lock().await.get_paths_list().await;
    Ok(Json(Payload::FilePaths(files)))
}

#[debug_handler]
pub async fn get_file(State(state): State<AppState>, uri: Uri) -> impl IntoResponse {
    let file_server = state.file_server.lock().await;
    let url_decoded_path = urlencoding::decode(&uri.to_string()).unwrap().into_owned();
    let path = file_server.get_file(&url_decoded_path).await;

    match path {
        Some(p) => {
            let req = Request::builder()
                .uri(uri.clone())
                .body(Body::empty())
                .unwrap();
            let f = ServeFile::new(format!("file_server/{p}"));
            f.oneshot(req)
                .await
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")))
        }
        None => Err((StatusCode::NOT_FOUND, format!("{uri} not found"))),
    }
}

#[debug_handler]
pub async fn add_files(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> Response<Payload> {
    let mut file_server = state.file_server.lock().await;

    #[derive(Debug)]
    struct AddFiles {
        directory: Option<String>,
        files: Vec<(String, Bytes)>,
    }
    let mut add_files = AddFiles {
        directory: None,
        files: Vec::new(),
    };

    while let Some(field) = multipart.next_field().await.unwrap() {
        if let Some(filename) = field.file_name() {
            let filename = filename.to_string();
            let bytes = match field.bytes().await {
                Ok(b) => b,
                Err(e) => {
                    return Err((
                        StatusCode::BAD_REQUEST,
                        Json(Payload::Error {
                            code: 5,
                            message: e.body_text(),
                        }),
                    ))
                }
            };
            info_span!("Add file ", filename);
            add_files.files.push((filename, bytes));
        } else if let Some(name) = field.name() {
            if name == "directory" {
                let directory = field
                    .text()
                    .await
                    .unwrap_or(String::new())
                    .trim()
                    .trim_end_matches("/")
                    .to_string();
                info_span!("Got directory name", directory);
                add_files.directory = Some(directory);
            }
            continue;
        } else {
            warn_span!("Unknown field", ?field);
        }
    }

    if let Some(dir) = add_files.directory {
        let mut files = Vec::with_capacity(add_files.files.len());

        // No files in request, create empty folder
        if add_files.files.is_empty() {
            info_span!("No files in request; creating dirs");
            match file_server.add_dir(&dir).await {
                Ok(_) => (),
                Err(message) => {
                    return Err((
                        StatusCode::BAD_REQUEST,
                        Json(Payload::Error { code: 3, message }),
                    ))
                }
            }
        }

        for (filename, bytes) in add_files.files {
            match file_server
                .add_file(format!("{dir}/{filename}"), bytes.len())
                .await
            {
                Ok(f) => files.push((f, bytes)),
                Err(message) => {
                    return Err((
                        StatusCode::BAD_REQUEST,
                        Json(Payload::Error { code: 4, message }),
                    ))
                }
            }
        }

        file_server.write().await;

        for (f, bytes) in files {
            let mut file = TokioFile::create(format!("file_server/{}", f.file_server))
                .await
                .unwrap();
            file.write_all(&bytes).await.unwrap();
        }

        Ok(Json(Payload::FilePaths(ListView(vec![]))))
    } else {
        error_span!("No directory field provided");
        Err((
            StatusCode::BAD_REQUEST,
            Json(Payload::Error {
                code: 2,
                message: format!("Directory cannot be empty"),
            }),
        ))
    }
}

#[derive(Deserialize, Debug, ToSchema, TS)]
#[ts(export, export_to = "api_bindings/files/")]
pub struct DeleteFilesRequest {
    /// path of files /dirs to be deleted.
    ///
    /// May not handle case where a folder and a file inside the folder is to be deleted in the same request
    ids: Vec<String>,
}

/// Delete files and directories
///
/// ids ending with a `'/'` will be treated as a dir, and recursively remote all contained items if present.
#[debug_handler]
pub async fn delete_files(
    State(state): State<AppState>,
    Json(files): Json<DeleteFilesRequest>,
) -> Response<Payload> {
    info_span!("Deleting files", ?files);
    let mut file_server = state.file_server.lock().await;

    //TODO: Don't interrupt on errors, let it delete all values, and then return all which did not succeed
    for id in files.ids {
        if id.ends_with('/') {
            match file_server.delete_dir(id).await {
                Ok(_) => (),
                Err(message) => {
                    return Err((
                        StatusCode::BAD_REQUEST,
                        Json(Payload::Error { code: 3, message }),
                    ))
                }
            }
        } else {
            match file_server.delete_file(id).await {
                Ok(_) => (),
                Err(message) => {
                    return Err((
                        StatusCode::BAD_REQUEST,
                        Json(Payload::Error { code: 4, message }),
                    ))
                }
            }
        }
    }

    Ok(Json(Payload::FilePaths(ListView(vec![]))))
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct File {
    name: String,
    /// Actual filename on disk
    ///
    /// `{UUID}.{ext}`
    file_server: String,
    /// File path through built file tree
    path: String,
    size: usize,
    date: DateTime<Local>,
}

#[derive(Clone, Debug)]
pub struct Directory {
    name: String,
    path: String,
    files: Arc<Mutex<Vec<File>>>,
    children: Arc<Mutex<Vec<Directory>>>,
}

#[derive(Deserialize, Serialize)]
struct DesDir {
    name: String,
    path: String,
    files: Vec<File>,
    children: Vec<Directory>,
}

impl<'de> Deserialize<'de> for Directory {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let input = DesDir::deserialize(deserializer)?;
        Ok(Self {
            name: input.name,
            path: input.path,
            files: Arc::new(Mutex::new(input.files)),
            children: Arc::new(Mutex::new(input.children)),
        })
    }
}

impl Serialize for Directory {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        DesDir::serialize(
            &DesDir {
                name: self.name.clone(),
                path: self.path.clone(),
                files: self.files.lock().unwrap().to_vec(),
                children: self.children.lock().unwrap().to_vec(),
            },
            serializer,
        )
    }
}

pub struct FileServer {
    con: AsyncMutex<MultiplexedConnection>,
    root: Directory,
}

impl FileServer {
    pub async fn new(redis_url: &str) -> Self {
        let client = Client::open(redis_url).unwrap();
        let mut con = client.get_multiplexed_tokio_connection().await.unwrap();

        let root = match con.json_get::<_, _, String>("files", ".").await {
            Ok(str) => serde_json::from_str(&str).unwrap(),
            Err(e) => {
                warn!("Could not parse files content, starting with a blank root directory (Error: {:?})", e);
                Directory {
                    name: "".to_string(),
                    path: String::from("/"),
                    files: Arc::new(Mutex::new(vec![])),
                    children: Arc::new(Mutex::new(vec![])),
                }
            }
        };

        Self {
            con: AsyncMutex::new(con),
            root,
        }
    }

    pub async fn get_paths_list(&self) -> ListView {
        (&self.root).into()
    }

    #[allow(unused)]
    pub async fn get_paths_tree(&self) -> TreeView {
        (&self.root).into()
    }

    /// Add file name to directory tree, and create folder if they don't already exists
    ///
    /// Does not call write to avoid writing when not all files are returning Ok()
    pub async fn add_file(&mut self, file_path: String, size: usize) -> Result<File, String> {
        info_span!("Adding file with ", file_path);
        let file_path = file_path.trim();
        let re = Regex::new(r"^/[\w/_\-\. ]+[\w]$").unwrap();
        if !re.is_match(file_path) {
            error_span!("Illegal file name");
            return Err("Illegal file name. Must only contain '_-./' special characters, start with root ('/') and end with a letter.".to_string());
        }

        let mut path = file_path
            .split('/')
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>();
        let file_name = path.pop().unwrap();

        let dir = self.create_up_to_dir(path).await;

        let mut files = dir.files.lock().unwrap();
        match files.binary_search_by_key(&file_name, |f| &f.name) {
            Ok(_) => Err(format!("File {file_path} already exists")),
            Err(pos) => {
                files.insert(
                    pos,
                    File {
                        name: file_name.to_string(),
                        file_server: format!(
                            "{}.{}",
                            Uuid::new_v4(),
                            Path::new(file_name).extension().unwrap().to_str().unwrap()
                        ),
                        path: file_path.to_string(),
                        size,
                        date: Local::now(),
                        //TODO: Add things like path to file on disk (with uuid generated name)
                    },
                );

                Ok(files[pos].clone())
            }
        }
    }

    pub async fn delete_file(&mut self, file_path: String) -> Result<File, String> {
        let file_path = file_path.trim();
        let mut path = file_path
            .split('/')
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>();
        let file_name = path.pop().unwrap();

        let dir = match self.traverse_to_dir(path).await {
            Some(d) => d,
            None => return Err(String::from("Directory does not exist")),
        };

        let file = {
            let mut files = dir.files.lock().unwrap();
            match files.binary_search_by_key(&file_name, |f| &f.name) {
                Ok(pos) => files.remove(pos),
                Err(_) => return Err(format!("File {file_path} does not exists")),
            }
        };

        match fs::remove_file(format!("file_server/{}", file.file_server)).await {
            Ok(_) => Ok(file),
            Err(e) => Err(e.to_string()),
        }
    }

    pub async fn add_dir(&mut self, dir_path: &String) -> Result<(), String> {
        info_span!("Adding dir ", dir_path);
        let dir_path = dir_path.trim();
        let re = Regex::new(r"^/[\w/_\- ]+[\w]$").unwrap();
        if !re.is_match(dir_path) {
            error_span!("Illegal dir name");
            return Err("Illegal directory name. Must only contain '_-' special characters, start with root ('/') and end with a letter.".to_string());
        }

        let path = dir_path
            .split('/')
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>();

        let _ = self.create_up_to_dir(path).await;

        Ok(())
    }

    pub async fn delete_dir(&mut self, dir_path: String) -> Result<Directory, String> {
        info_span!("Deleting dir", dir_path);
        let dir_path = dir_path.trim();
        let mut path = dir_path
            .split('/')
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>();

        let dir_name = match path.pop() {
            Some(d) => d,
            None => return Err("Will not delete root folder".into()),
        };

        let parent_dir = match self.traverse_to_dir(path).await {
            Some(d) => d,
            None => {
                error_span!("Parent directory does not exist");
                return Err(String::from("Parent directory does not exist"));
            }
        };
        let dir = {
            let mut dirs = parent_dir.children.lock().unwrap();
            match dirs.binary_search_by_key(&dir_name, |d| &d.name) {
                Ok(pos) => dirs.remove(pos),
                Err(_) => {
                    error_span!("Directory does not exist");
                    return Err(String::from("Directory does not exist"));
                }
            }
        };

        let mut files = vec![];
        let mut stack = VecDeque::from([dir.clone()]);
        while let Some(dir) = stack.pop_front() {
            files.append(&mut dir.files.lock().unwrap());
            stack.extend(std::mem::take(&mut *dir.children.lock().unwrap()).into_iter());
        }

        for f in files {
            fs::remove_file(format!("file_server/{}", f.file_server))
                .await
                .unwrap();
        }
        Ok(dir)
    }

    async fn get_file(&self, file_path: &String) -> Option<String> {
        let mut path = file_path
            .split('/')
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>();
        let file_name = path.pop()?;

        let dir = self.traverse_to_dir(path).await?;

        let files = dir.files.lock().unwrap();
        match files.binary_search_by_key(&file_name, |f| &f.name) {
            Ok(pos) => Some(files[pos].file_server.clone()),
            Err(_) => None,
        }
    }

    //TODO: traversing where no folder is created, error is returned, and where both file path and dir path works as input.

    /// Traverse through tree until path and create dirs on the way
    async fn create_up_to_dir(&self, path: Vec<&str>) -> Directory {
        let mut dir = self.root.clone();
        let mut depth = 1;
        for p in &path {
            let dir_c = dir.clone();
            let mut d = dir_c.children.lock().unwrap();
            let pos = match d.binary_search_by_key(p, |d| &d.name) {
                // Update dir to dir and traverse down the tree
                Ok(pos) => pos,
                // Add Dir at sorted position in Vec if not present
                Err(pos) => {
                    d.insert(
                        pos,
                        Directory {
                            name: p.to_string(),
                            path: path
                                .iter()
                                .take(depth)
                                .fold(String::from(""), |acc, x| acc + "/" + x)
                                .to_owned(),
                            files: Arc::new(Mutex::new(vec![])),
                            children: Arc::new(Mutex::new(vec![])),
                        },
                    );
                    pos
                }
            };
            dir = d.get(pos).unwrap().clone();
            depth += 1;
        }
        dir
    }

    /// Traverse through tree until path. Returns None if path does not exist
    async fn traverse_to_dir(&self, path: Vec<&str>) -> Option<Directory> {
        let mut dir = self.root.clone();
        for p in &path {
            let dir_c = dir.clone();
            let d = dir_c.children.lock().unwrap();
            let pos = match d.binary_search_by_key(p, |d| &d.name) {
                // Update dir to dir and traverse down the tree
                Ok(pos) => pos,
                // Add Dir at sorted position in Vec if not present
                Err(_) => return None,
            };
            dir = d.get(pos).unwrap().clone();
        }
        Some(dir)
    }

    async fn write(&mut self) {
        let root_dir = &self.root.clone();
        if let Err(error) = self
            .con
            .lock()
            .await
            .json_set::<_, _, _, String>("files", "$", &root_dir)
            .await
        {
            error_span!("Redis Error", ?error);
            // error_span!("Logging current state instead", ?self.content);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn traverse() {}
}
