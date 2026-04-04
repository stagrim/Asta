use std::{
    collections::{LinkedList, VecDeque},
    ffi::OsString,
    path::Path,
    sync::{Arc, LazyLock, Mutex},
};

use axum::{
    Json,
    body::Body,
    extract::{DefaultBodyLimit, Multipart, State},
    response::IntoResponse,
};
use axum_macros::debug_handler;
use chrono::{DateTime, Local};
use hyper::{Request, StatusCode, Uri};
#[cfg(not(test))]
use redis::{Client, JsonAsyncCommands, aio::MultiplexedConnection};
use regex::Regex;
use serde::{Deserialize, Serialize};
#[cfg(not(test))]
use tokio::sync::Mutex as AsyncMutex;
use tokio::{
    fs::{self, File as TokioFile},
    io::AsyncWriteExt,
};
use tower::ServiceExt;
use tower_http::services::ServeFile;
#[cfg(not(test))]
use tracing::warn;
use tracing::{error_span, info_span, warn_span};
use ts_rs::TS;
use utoipa::{
    ToSchema,
    openapi::{ArrayBuilder, Ref, RefOr, Schema},
};
use utoipa_axum::{router::OpenApiRouter, routes};
use uuid::Uuid;

use crate::AppState;

pub fn file_api_router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(get_all_paths_tree))
        .routes(routes!(get_all_paths_list))
        .routes(routes!(delete_files))
        .routes(routes!(add_files))
        .layer(DefaultBodyLimit::max(100_000_000))
}

pub type Response<T> = Result<Json<T>, (StatusCode, Json<T>)>;

#[derive(Serialize, ToSchema, TS)]
#[serde(tag = "type", content = "content")]
#[ts(export, export_to = "api_bindings/files/")]
pub enum Payload {
    FilePaths(ListView),
    Error { code: u8, message: String },
}

#[derive(Deserialize, Serialize, Debug, ToSchema, TS)]
#[ts(export, export_to = "api_bindings/files/")]
pub struct TreeFile {
    id: String,
    name: String,
    size: usize,
    date: String,
}

/// Helper function to handle recursion for Vec<TreeDirectory>
fn recursive_directory_schema() -> RefOr<Schema> {
    Schema::Array(
        ArrayBuilder::new()
            .items(Ref::from_schema_name("TreeDirectory"))
            .build(),
    )
    .into()
}

#[derive(Deserialize, Serialize, Debug, ToSchema, TS)]
#[ts(export, export_to = "api_bindings/files/")]
pub struct TreeDirectory {
    id: String,
    name: String,
    files: Vec<TreeFile>,
    // Manually specify the schema to break infinite recursion
    #[schema(schema_with = recursive_directory_schema)]
    directories: Vec<TreeDirectory>,
}

impl From<&File> for TreeFile {
    fn from(value: &File) -> Self {
        TreeFile {
            id: value.path.clone(),
            name: value.name.clone(),
            size: value.size,
            date: value.date.to_rfc3339(),
        }
    }
}

impl From<&Directory> for TreeDirectory {
    fn from(value: &Directory) -> Self {
        TreeDirectory {
            id: format!("{}/", value.path.clone()),
            name: value.name.clone(),
            files: value
                .files
                .lock()
                .unwrap()
                .iter()
                .map(|f| f.into())
                .collect::<Vec<_>>(),
            directories: value
                .children
                .lock()
                .unwrap()
                .iter()
                .map(|f| f.into())
                .collect::<Vec<_>>(),
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
    r#type: ListViewItemType,
}

#[derive(Serialize, Deserialize, Debug, ToSchema, TS)]
#[ts(export, export_to = "api_bindings/files/")]
enum ListViewItemType {
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
            r#type: ListViewItemType::File,
        }
    }
}
impl From<&Directory> for ListViewItem {
    fn from(value: &Directory) -> Self {
        ListViewItem {
            id: value.path.clone(),
            // TODO: show sum of size of files here, or items
            size: 0,
            // TODO: show latest file change here
            date: "1996-12-19T16:39:57-08:00".to_string(),
            r#type: ListViewItemType::Directory,
        }
    }
}

#[utoipa::path(
    get,
    path = "/list",
    tag = "files",
    responses(
        (status = 200, description = "List all files flat", body = Payload)
    )
)]
pub async fn get_all_paths_list(State(state): State<AppState>) -> Response<Payload> {
    let files = state.file_server.lock().await.get_paths_list().await;
    Ok(Json(Payload::FilePaths(files)))
}

#[utoipa::path(
    get,
    path = "/tree",
    tag = "files",
    responses(
        (status = 200, description = "Get file tree", body = TreeDirectory)
    )
)]
pub async fn get_all_paths_tree(State(state): State<AppState>) -> Response<TreeDirectory> {
    let files = state.file_server.lock().await.get_paths_tree().await;
    Ok(Json(files))
}

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

/// Struct representing the multipart/form-data schema for file uploads
#[derive(ToSchema)]
pub struct FileUpload {
    /// Target directory to upload files to
    directory: String,
    /// One or more files to upload
    #[schema(value_type = Vec<String>, format = Binary)]
    files: Vec<FileItem>,
}

struct FileItem {
    pub name: String,
    pub content: Vec<u8>,
}

impl FileUpload {
    /// Parse Multipart stream into FileUpload struct
    pub async fn from_multipart(mut multipart: Multipart) -> Result<Self, (u8, String)> {
        let mut directory = None;
        let mut files = Vec::new();

        while let Some(field) = multipart.next_field().await.unwrap() {
            if let Some(filename) = field.file_name() {
                let filename = filename.to_string();
                let bytes = match field.bytes().await {
                    Ok(b) => b.into_iter().collect::<Vec<_>>(),
                    Err(e) => return Err((5, e.body_text())),
                };
                info_span!("Add file ", filename);
                files.push(FileItem {
                    name: filename,
                    content: bytes,
                });
            } else if let Some(name) = field.name()
                && name == "directory"
            {
                let dir = field
                    .text()
                    .await
                    .unwrap_or(String::new())
                    .trim()
                    .trim_end_matches("/")
                    .to_string();
                info_span!("Got directory name", dir);
                directory = Some(dir);
            } else {
                warn_span!("Unknown field", ?field);
            }
        }

        match directory {
            Some(directory) => Ok(FileUpload { directory, files }),
            None => Err((2, "Directory field cannot be empty".to_string())),
        }
    }
}

#[utoipa::path(
    post,
    path = "/",
    tag = "files",
    request_body(content = FileUpload, content_type = "multipart/form-data"),
    responses(
        (status = 200, description = "Files uploaded successfully", body = Payload),
        (status = 400, description = "Bad Request (e.g. missing directory field)", body = Payload)
    )
)]
#[debug_handler]
pub async fn add_files(State(state): State<AppState>, multipart: Multipart) -> Response<Payload> {
    let mut file_server = state.file_server.lock().await;

    let upload = match FileUpload::from_multipart(multipart).await {
        Ok(u) => u,
        Err((code, message)) => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(Payload::Error { code, message }),
            ));
        }
    };

    // No files in request, create empty folder
    if upload.files.is_empty() {
        info_span!("No files in request; creating dirs");
        match file_server.add_dir(&upload.directory).await {
            Ok(_) => (),
            Err(message) => {
                return Err((
                    StatusCode::BAD_REQUEST,
                    Json(Payload::Error { code: 3, message }),
                ));
            }
        }
    }

    let mut errors = Vec::new();

    for file_item in upload.files {
        match file_server
            .add_file(
                format!("{}/{}", upload.directory, file_item.name),
                file_item.content.len(),
            )
            .await
        {
            Ok(f) => {
                match TokioFile::create(format!("file_server/{}", f.file_server)).await {
                    Ok(mut file) => {
                        if let Err(e) = file.write_all(&file_item.content).await {
                            errors.push(e.to_string());
                        }
                    }
                    Err(e) => errors.push(e.to_string()),
                };
            }
            Err(message) => errors.push(message),
        }
    }

    file_server.write().await;

    if errors.is_empty() {
        Ok(Json(Payload::FilePaths(ListView(vec![]))))
    } else {
        Err((
            StatusCode::BAD_REQUEST,
            Json(Payload::Error {
                code: 4,
                message: errors.join(", "),
            }),
        ))
    }
}

#[derive(Serialize, Deserialize, Debug, ToSchema, TS)]
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
#[utoipa::path(
    delete,
    path = "/",
    tag = "files",
    request_body = DeleteFilesRequest,
    responses(
        (status = 200, description = "Files deleted successfully", body = Payload),
        (status = 400, description = "Bad Request", body = Payload)
    )
)]
#[debug_handler]
pub async fn delete_files(
    State(state): State<AppState>,
    Json(files): Json<DeleteFilesRequest>,
) -> Response<Payload> {
    info_span!("Deleting files", ?files);
    let mut file_server = state.file_server.lock().await;

    let mut errors = Vec::new();

    //TODO: Don't interrupt on errors, let it delete all values, and then return all which did not succeed
    for id in files.ids {
        if id.ends_with('/') {
            match file_server.delete_dir(id).await {
                Ok(_) => (),
                Err(message) => errors.push(message),
            }
        } else {
            match file_server.delete_file(id).await {
                Ok(_) => (),
                Err(message) => errors.push(message),
            }
        }
    }

    file_server.write().await;

    if errors.is_empty() {
        Ok(Json(Payload::FilePaths(ListView(vec![]))))
    } else {
        Err((
            StatusCode::BAD_REQUEST,
            Json(Payload::Error {
                code: 3,
                message: errors.join(", "),
            }),
        ))
    }
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
    #[cfg(not(test))]
    con: AsyncMutex<MultiplexedConnection>,
    root: Directory,
}

pub static FILE_PATH_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^/[\w/_\-\. ]+[\w]$").unwrap());

pub static DIR_REGEX: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^/[\w/_\- ]+[\w]$").unwrap());

impl FileServer {
    #[cfg(not(test))]
    pub async fn new(redis_url: &str) -> Self {
        let client = Client::open(redis_url).unwrap();
        let mut con = client.get_multiplexed_tokio_connection().await.unwrap();

        let root = match con.json_get::<_, _, String>("files", ".").await {
            Ok(str) => serde_json::from_str(&str).unwrap(),
            Err(e) => {
                warn!(
                    "Could not parse files content, starting with a blank root directory (Error: {:?})",
                    e
                );
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

    #[cfg(test)]
    pub async fn new(_redis_url: &str) -> Self {
        // Return a fresh, empty in-memory tree for every test
        Self {
            root: Directory {
                name: "".to_string(),
                path: String::from("/"),
                files: Arc::new(Mutex::new(vec![])),
                children: Arc::new(Mutex::new(vec![])),
            },
        }
    }

    pub async fn get_paths_list(&self) -> ListView {
        (&self.root).into()
    }

    pub async fn get_paths_tree(&self) -> TreeDirectory {
        (&self.root).into()
    }

    /// Add file name to directory tree, and create folder if they don't already exists
    ///
    /// Does not call write to avoid writing when not all files are returning Ok()
    pub async fn add_file(&mut self, file_path: String, size: usize) -> Result<File, String> {
        info_span!("Adding file with ", file_path);
        let file_path = file_path.trim();
        if !FILE_PATH_REGEX.is_match(file_path) {
            error_span!("Illegal file name");
            return Err("Illegal file name. Must only contain '_-./' special characters, start with root ('/') and end with a letter.".to_string());
        }

        let mut path = file_path
            .split('/')
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>();
        let file_path = format!("/{}", path.join("/"));
        let file_name = path.pop().unwrap();

        let dir = self.create_up_to_dir(&path);

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
                            Path::new(file_name)
                                .extension()
                                .unwrap_or(&OsString::from("txt"))
                                .to_str()
                                .unwrap()
                        ),
                        path: file_path,
                        size,
                        date: Local::now(),
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

        let dir = match self.traverse_to_dir(&path) {
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
        if !DIR_REGEX.is_match(dir_path) {
            error_span!("Illegal dir name");
            return Err("Illegal directory name. Must only contain '_-' special characters, start with root ('/') and end with a letter.".to_string());
        }

        let path = dir_path
            .split('/')
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>();

        let _ = self.create_up_to_dir(&path);

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

        let parent_dir = match self.traverse_to_dir(&path) {
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

        let dir = self.traverse_to_dir(&path)?;

        let files = dir.files.lock().unwrap();
        match files.binary_search_by_key(&file_name, |f| &f.name) {
            Ok(pos) => Some(files[pos].file_server.clone()),
            Err(_) => None,
        }
    }

    pub async fn move_file(
        &self,
        file_path: &String,
        new_file_path: &String,
    ) -> Result<File, String> {
        let file_path = file_path.trim();
        let new_file_path = new_file_path.trim();
        if !FILE_PATH_REGEX.is_match(new_file_path) {
            error_span!("Illegal file name for new file");
            return Err("Illegal file name for new file. Must only contain '_-./' special characters, start with root ('/') and end with a letter.".to_string());
        }

        let mut path = file_path
            .split('/')
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>();
        let file_name = path.pop().unwrap();

        let dir = match self.traverse_to_dir(&path) {
            Some(d) => d,
            None => return Err(String::from("Directory does not exist")),
        };

        let mut new_path = new_file_path
            .split('/')
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>();
        let new_file_name = new_path.pop().unwrap();

        let move_to_dir = match self.traverse_to_dir(&new_path) {
            Some(d) => d,
            None => return Err(String::from("Directory does not exist")),
        };

        let mut old_dir_files = dir.files.lock().unwrap();
        let old_dir_file_pos = match old_dir_files.binary_search_by_key(&file_name, |f| &f.name) {
            Ok(pos) => pos,
            Err(_) => return Err(format!("File {file_path} does not exist")),
        };

        if move_to_dir.path == dir.path {
            match old_dir_files.binary_search_by_key(&new_file_name, |f| &f.name) {
                Ok(_) => Err(format!(
                    "Cannot move file since {new_file_path} already exists"
                )),
                Err(pos) => {
                    let mut file = old_dir_files.remove(old_dir_file_pos);
                    file.path = new_file_path.to_string();
                    file.name = new_file_name.to_string();

                    // Decrease insert pos by on if file will be inserted after the before position
                    // to account for itself being removed earlier in the array
                    let insert_pos = if old_dir_file_pos < pos { pos - 1 } else { pos };
                    old_dir_files.insert(insert_pos, file.clone());

                    Ok(file)
                }
            }
        } else {
            let mut move_to_dir_files = move_to_dir.files.lock().unwrap();
            match move_to_dir_files.binary_search_by_key(&new_file_name, |f| &f.name) {
                Ok(_) => Err(format!(
                    "Cannot move file since {new_file_path} already exists"
                )),
                Err(pos) => {
                    let mut file = old_dir_files.remove(old_dir_file_pos);
                    file.path = new_file_path.to_string();
                    file.name = new_file_name.to_string();

                    move_to_dir_files.insert(pos, file.clone());

                    Ok(file)
                }
            }
        }
    }

    pub async fn move_dir(
        &self,
        dir_path: &String,
        new_dir_path: &String,
    ) -> Result<Directory, String> {
        info_span!("Moving dir", dir_path, new_dir_path);

        let old_dir_path = dir_path.trim();
        let new_dir_path = new_dir_path.trim();
        if !DIR_REGEX.is_match(new_dir_path) {
            error_span!("Illegal dir name");
            return Err("Illegal directory name. Must only contain '_-' special characters, start with root ('/') and end with a letter.".to_string());
        }

        if new_dir_path == old_dir_path || new_dir_path.starts_with(&format!("{}/", old_dir_path)) {
            error_span!("Cannot move directory into itself");
            return Err("Cannot move a directory into itself or its subdirectories.".to_string());
        }

        let mut old_path = old_dir_path
            .split('/')
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>();

        let old_dir_name = match old_path.pop() {
            Some(d) => d,
            None => return Err("Will not move root folder".into()),
        };
        let old_parent_dir = match self.traverse_to_dir(&old_path) {
            Some(d) => d,
            None => {
                error_span!("Parent directory does not exist");
                return Err(String::from("Directory does not exist"));
            }
        };

        let mut new_path = new_dir_path
            .split('/')
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>();
        let new_dir_name = match new_path.pop() {
            Some(d) => d,
            None => return Err("Will not move root folder".into()),
        };
        let new_parent_dir = match self.traverse_to_dir(&new_path) {
            Some(d) => d,
            None => {
                error_span!("Parent directory does not exist");
                return Err(String::from("Directory does not exist"));
            }
        };

        let mut old_parent_dir_dirs = old_parent_dir.children.lock().unwrap();
        let old_parent_dir_pos =
            match old_parent_dir_dirs.binary_search_by_key(&old_dir_name, |d| &d.name) {
                Ok(pos) => pos,
                Err(_) => return Err(format!("Directory {old_dir_path} does not exist")),
            };

        let dir = if old_parent_dir.path == new_parent_dir.path {
            match old_parent_dir_dirs.binary_search_by_key(&new_dir_name, |d| &d.name) {
                Ok(_) => {
                    return Err(format!(
                        "Cannot move directory since {new_dir_path} already exists"
                    ));
                }
                Err(pos) => {
                    let mut dir = old_parent_dir_dirs.remove(old_parent_dir_pos);
                    dir.path = new_dir_path.to_string();
                    dir.name = new_dir_name.to_string();

                    // Decrease insert pos by on if dir will be inserted after the before position
                    // to account for itself being removed earlier in the array
                    let insert_pos = if old_parent_dir_pos < pos {
                        pos - 1
                    } else {
                        pos
                    };
                    old_parent_dir_dirs.insert(insert_pos, dir.clone());

                    dir
                }
            }
        } else {
            let mut move_to_dir_dirs = new_parent_dir.children.lock().unwrap();
            match move_to_dir_dirs.binary_search_by_key(&new_dir_name, |f| &f.name) {
                Ok(_) => {
                    return Err(format!(
                        "Cannot move directory since {new_dir_path} already exists"
                    ));
                }
                Err(pos) => {
                    let mut dir = old_parent_dir_dirs.remove(old_parent_dir_pos);
                    dir.path = new_dir_path.to_string();
                    dir.name = new_dir_name.to_string();
                    move_to_dir_dirs.insert(pos, dir.clone());

                    dir
                }
            }
        };
        Self::recursivly_update_path(&dir);
        Ok(dir)
    }

    fn recursivly_update_path(dir: &Directory) {
        for d in dir.children.lock().unwrap().iter_mut() {
            d.path = format!("{}/{}", dir.path, d.name);
            Self::recursivly_update_path(d);
        }
        for f in dir.files.lock().unwrap().iter_mut() {
            f.path = format!("{}/{}", dir.path, f.name)
        }
    }

    /// Traverse through tree until path and create dirs on the way
    fn create_up_to_dir(&self, path: &[&str]) -> Directory {
        let mut dir = self.root.clone();
        let mut depth = 1;
        for p in path {
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
    fn traverse_to_dir(&self, path: &[&str]) -> Option<Directory> {
        let mut dir = self.root.clone();
        for p in path {
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

    #[cfg(test)]
    async fn write(&mut self) {
        // Do not write to any db when in test environment
    }

    #[cfg(not(test))]
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
    use crate::store::store::Store;

    use super::*;
    use axum::serve;
    use reqwest::multipart;
    use std::sync::Arc;
    use tokio::fs;
    use tokio::net::TcpListener;
    use tokio::sync::Mutex as AsyncMutex;

    /// Helper function
    async fn setup_test_server() -> FileServer {
        let _ = fs::create_dir_all("file_server").await;

        FileServer::new("not used since in test environment").await
    }

    #[tokio::test]
    async fn test_regex_path_validation() {
        // Valid paths
        assert!(FILE_PATH_REGEX.is_match("/file.txt"));
        assert!(FILE_PATH_REGEX.is_match("/folder/file.txt"));
        assert!(FILE_PATH_REGEX.is_match("/folder 1/my_file-name.txt"));
        assert!(FILE_PATH_REGEX.is_match("/deeply/nested/dir/file.ts"));

        // Invalid paths
        assert!(!FILE_PATH_REGEX.is_match("file.txt")); // Missing leading slash
        assert!(!FILE_PATH_REGEX.is_match("/folder/")); // Ends in slash (not a file)
        assert!(!FILE_PATH_REGEX.is_match("/fol@der/file.txt")); // Illegal characters
    }

    #[tokio::test]
    async fn test_add_file_creates_directories() {
        let mut server = setup_test_server().await;

        let path = "/test_folder/nested/file.txt".to_string();
        let file = server
            .add_file(path.clone(), 1024)
            .await
            .expect("Failed to add file");

        assert_eq!(file.name, "file.txt");
        assert_eq!(file.path, "/test_folder/nested/file.txt");
        assert_eq!(file.size, 1024);

        let tree = server.get_paths_tree().await;

        let test_folder = tree
            .directories
            .iter()
            .find(|d| d.name == "test_folder")
            .unwrap();
        let nested = test_folder
            .directories
            .iter()
            .find(|d| d.name == "nested")
            .unwrap();

        assert_eq!(nested.files.len(), 1);
        assert_eq!(nested.files[0].name, "file.txt");
    }

    #[tokio::test]
    async fn test_add_duplicate_file_fails() {
        let mut server = setup_test_server().await;

        let path = "/duplicates/file.txt".to_string();

        let _ = server.add_file(path.clone(), 100).await.unwrap();

        let result = server.add_file(path.clone(), 200).await;
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            "File /duplicates/file.txt already exists"
        );
    }

    #[tokio::test]
    async fn test_delete_file_success_and_failure() {
        let mut server = setup_test_server().await;
        let path = "/to_delete/delete_me.txt".to_string();

        let file = server.add_file(path.clone(), 10).await.unwrap();

        let disk_path = format!("file_server/{}", file.file_server);
        fs::write(&disk_path, b"dummy data").await.unwrap();

        let deleted = server
            .delete_file(path.clone())
            .await
            .expect("Failed to delete file");
        assert_eq!(deleted.name, "delete_me.txt");

        assert!(!Path::new(&disk_path).exists());

        let fail_result = server
            .delete_file("/to_delete/does_not_exist.txt".to_string())
            .await;
        assert!(fail_result.is_err());
        assert_eq!(
            fail_result.unwrap_err(),
            "File /to_delete/does_not_exist.txt does not exists"
        );
    }

    #[tokio::test]
    async fn test_add_and_delete_directory() {
        let mut server = setup_test_server().await;

        let dir_path = "/my_folder/child_folder".to_string();
        server.add_dir(&dir_path).await.expect("Failed to add dir");

        let file = server
            .add_file("/my_folder/child_folder/test.txt".to_string(), 0)
            .await
            .unwrap();
        let disk_path = format!("file_server/{}", file.file_server);
        fs::write(&disk_path, b"data").await.unwrap();

        let deleted_dir = server
            .delete_dir("/my_folder".to_string())
            .await
            .expect("Failed to delete dir");
        assert_eq!(deleted_dir.name, "my_folder");

        assert!(!Path::new(&disk_path).exists());

        let tree = server.get_paths_tree().await;
        assert!(tree.directories.iter().all(|d| d.name != "my_folder"));
    }

    #[tokio::test]
    async fn test_delete_root_directory_fails() {
        let mut server = setup_test_server().await;

        // Attempting to delete "/" should be blocked
        let result = server.delete_dir("/".to_string()).await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Will not delete root folder");
    }

    #[tokio::test]
    async fn test_virtual_path_weirdness_get_normalized() {
        let mut server = setup_test_server().await;

        let weird_path = "///weird////path//file.txt".to_string();

        let file = server.add_file(weird_path, 50).await.unwrap();
        assert_eq!(file.path, "/weird/path/file.txt");

        let tree = server.get_paths_tree().await;
        let weird_dir = tree.directories.iter().find(|d| d.name == "weird").unwrap();
        let path_dir = weird_dir
            .directories
            .iter()
            .find(|d| d.name == "path")
            .unwrap();

        assert_eq!(path_dir.files[0].name, "file.txt");

        // Easier to make harmless wonkyness a feature than fixing it...
        let weird_path = "/../wonky/.file/....path..txt".to_string();

        let file = server.add_file(weird_path, 50).await.unwrap();
        assert_eq!(file.path, "/../wonky/.file/....path..txt");
    }

    #[tokio::test]
    async fn test_move_file_cross_directory_and_rename() {
        let mut server = setup_test_server().await;
        server.add_dir(&"/folder_a".to_string()).await.unwrap();
        server.add_dir(&"/folder_b".to_string()).await.unwrap();
        server
            .add_file("/folder_a/test.txt".to_string(), 10)
            .await
            .unwrap();

        // Move and rename at the same time
        let moved_file = server
            .move_file(
                &"/folder_a/test.txt".to_string(),
                &"/folder_b/moved.txt".to_string(),
            )
            .await
            .expect("Failed to move file");

        assert_eq!(moved_file.path, "/folder_b/moved.txt");
        assert_eq!(moved_file.name, "moved.txt");

        let tree = server.get_paths_tree().await;
        let folder_a = tree
            .directories
            .iter()
            .find(|d| d.name == "folder_a")
            .unwrap();
        assert!(
            folder_a.files.is_empty(),
            "File should be removed from old directory"
        );

        let folder_b = tree
            .directories
            .iter()
            .find(|d| d.name == "folder_b")
            .unwrap();
        assert_eq!(folder_b.files.len(), 1);
        assert_eq!(
            folder_b.files[0].name, "moved.txt",
            "File should exist in new directory"
        );
    }

    #[tokio::test]
    async fn test_move_file_rename_index_shift_bug() {
        let mut server = setup_test_server().await;
        server.add_dir(&"/docs".to_string()).await.unwrap();
        server
            .add_file("/docs/apple.txt".to_string(), 10)
            .await
            .unwrap();
        server
            .add_file("/docs/banana.txt".to_string(), 10)
            .await
            .unwrap();
        server
            .add_file("/docs/zebra.txt".to_string(), 10)
            .await
            .unwrap();

        // Rename apple to carrot. It must be inserted between banana and zebra.
        // If the index shift bug isn't fixed, it will break the alphabetical order.
        server
            .move_file(
                &"/docs/apple.txt".to_string(),
                &"/docs/carrot.txt".to_string(),
            )
            .await
            .unwrap();

        let tree = server.get_paths_tree().await;
        let docs = tree.directories.iter().find(|d| d.name == "docs").unwrap();

        assert_eq!(docs.files.len(), 3);
        assert_eq!(docs.files[0].name, "banana.txt");
        assert_eq!(docs.files[1].name, "carrot.txt");
        assert_eq!(docs.files[2].name, "zebra.txt");
    }

    #[tokio::test]
    async fn test_move_file_collisions_and_errors() {
        let mut server = setup_test_server().await;
        server
            .add_file("/docs/file1.txt".to_string(), 10)
            .await
            .unwrap();
        server
            .add_file("/docs/file2.txt".to_string(), 10)
            .await
            .unwrap();
        server
            .add_file("/archive/file1.txt".to_string(), 10)
            .await
            .unwrap();

        // 1. Same directory collision
        let err1 = server
            .move_file(
                &"/docs/file1.txt".to_string(),
                &"/docs/file2.txt".to_string(),
            )
            .await
            .unwrap_err();
        assert!(err1.contains("already exists"));

        // 2. Cross directory collision
        let err2 = server
            .move_file(
                &"/docs/file1.txt".to_string(),
                &"/archive/file1.txt".to_string(),
            )
            .await
            .unwrap_err();
        assert!(err2.contains("already exists"));

        // 3. Source does not exist
        let err3 = server
            .move_file(
                &"/docs/ghost.txt".to_string(),
                &"/archive/ghost.txt".to_string(),
            )
            .await
            .unwrap_err();
        assert!(err3.contains("does not exist"));
    }

    #[tokio::test]
    async fn test_move_dir_recursive_path_updates() {
        let mut server = setup_test_server().await;
        server
            .add_file("/parent/child/deep/file.txt".to_string(), 10)
            .await
            .unwrap();
        server.add_dir(&"/archive".to_string()).await.unwrap();

        let moved_dir = server
            .move_dir(
                &"/parent/child".to_string(),
                &"/archive/renamed_child".to_string(),
            )
            .await
            .unwrap();

        assert_eq!(moved_dir.path, "/archive/renamed_child");

        let tree = server.get_paths_tree().await;
        let archive = tree
            .directories
            .iter()
            .find(|d| d.name == "archive")
            .unwrap();
        let renamed = archive
            .directories
            .iter()
            .find(|d| d.name == "renamed_child")
            .unwrap();
        let deep = renamed
            .directories
            .iter()
            .find(|d| d.name == "deep")
            .unwrap();

        // Verify that internal properties propagated through the whole tree branch!
        // (TreeDirectory maps value.path to `id` with a trailing slash, and TreeFile maps value.path directly to `id`)
        assert_eq!(deep.id, "/archive/renamed_child/deep/");
        assert_eq!(deep.files.len(), 1);
        assert_eq!(deep.files[0].id, "/archive/renamed_child/deep/file.txt");
    }

    #[tokio::test]
    async fn test_move_dir_inception_protection() {
        let mut server = setup_test_server().await;
        server.add_dir(&"/docs/archive".to_string()).await.unwrap();

        // 1. Block moving into itself
        let err1 = server
            .move_dir(&"/docs".to_string(), &"/docs".to_string())
            .await
            .unwrap_err();
        assert!(err1.contains("Cannot move a directory into itself"));

        // 2. Block moving into its own child (Orphan Tree Bug)
        let err2 = server
            .move_dir(&"/docs".to_string(), &"/docs/archive/nested".to_string())
            .await
            .unwrap_err();
        assert!(err2.contains("Cannot move a directory into itself"));

        // 3. DO NOT block moving into a different folder with a similar prefix name!
        // This ensures new_dir_path.starts_with(&format!("{}/", old_dir_path)) is working perfectly.
        server.add_dir(&"/docs_new".to_string()).await.unwrap();
        let success = server
            .move_dir(&"/docs".to_string(), &"/docs_new/docs".to_string())
            .await;

        assert!(
            success.is_ok(),
            "Failed to move into similarly named sibling directory"
        );
    }

    #[tokio::test]
    async fn test_full_api_upload_read_delete() {
        let file_server = setup_test_server().await;

        let state = AppState {
            file_server: Arc::new(AsyncMutex::new(file_server)),
            htmx_hash: String::new(),
            store: Arc::new(Store::new("again, not used in test environment").await),
        };

        let (app, _api) = file_api_router().with_state(state).split_for_parts();

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let base_url = format!("http://{}", addr);

        tokio::spawn(async move {
            serve(listener, app).await.unwrap();
        });

        let client = reqwest::Client::new();

        // ==========================================
        // TEST A: UPLOAD FILE (POST /)
        // ==========================================
        let file_content = b"Hello, Axum Web API!".to_vec();
        let part = multipart::Part::bytes(file_content.clone())
            .file_name("api_test.txt")
            .mime_str("text/plain")
            .unwrap();

        let form = multipart::Form::new()
            .text("directory", "/api_folder")
            .part("files", part);

        let upload_res = client
            .post(&format!("{}/", base_url))
            .multipart(form)
            .send()
            .await
            .expect("Failed to send upload request");

        assert_eq!(upload_res.status(), StatusCode::OK);

        // ==========================================
        // TEST B: READ TREE (GET /tree)
        // ==========================================
        let tree_res = client
            .get(&format!("{}/tree", base_url))
            .send()
            .await
            .expect("Failed to fetch tree");

        assert_eq!(tree_res.status(), StatusCode::OK);

        let tree_json: TreeDirectory = tree_res.json().await.unwrap();

        let api_folder = tree_json
            .directories
            .iter()
            .find(|d| d.name == "api_folder")
            .unwrap();

        assert_eq!(api_folder.files.len(), 1);
        assert_eq!(api_folder.files[0].name, "api_test.txt");
        assert_eq!(api_folder.files[0].size, 20); // Length of "Hello, Axum Web API!"

        // ==========================================
        // TEST C: DELETE FILE (DELETE /)
        // ==========================================
        let delete_payload = DeleteFilesRequest {
            ids: vec!["/api_folder/api_test.txt".to_string()],
        };

        let delete_res = client
            .delete(&format!("{}/", base_url))
            .json(&delete_payload)
            .send()
            .await
            .expect("Failed to send delete request");

        assert_eq!(delete_res.status(), StatusCode::OK);

        let final_tree_res = client
            .get(&format!("{}/tree", base_url))
            .send()
            .await
            .unwrap();

        let final_tree_json: TreeDirectory = final_tree_res.json().await.unwrap();
        let final_api_folder = final_tree_json
            .directories
            .iter()
            .find(|d| d.name == "api_folder")
            .unwrap();

        assert_eq!(final_api_folder.files.len(), 0);
    }
}
