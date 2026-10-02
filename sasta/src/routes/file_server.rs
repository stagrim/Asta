use std::{collections::VecDeque, sync::Arc, time::Duration};

use futures_util::future::BoxFuture;

use axum::{
    Json,
    body::Body,
    extract::{DefaultBodyLimit, Multipart, State},
    response::IntoResponse,
};
use axum_macros::debug_handler;
use hyper::{Request, StatusCode, Uri};
use serde::{Deserialize, Serialize};
use tower::ServiceExt;
use tower_http::services::ServeFile;
use tracing::{info_span, warn_span};
use utoipa::{
    ToSchema,
    openapi::{ArrayBuilder, Ref, RefOr, Schema},
};
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::{
    AppState,
    file_server::file_server::{Directory, File, FileServer, FileServerError, FileTransaction},
};

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
    pub async fn from_multipart(mut multipart: Multipart) -> Result<Self, String> {
        let mut directory = None;
        let mut files = Vec::new();

        while let Some(field) = multipart.next_field().await.unwrap() {
            if let Some(filename) = field.file_name() {
                let filename = filename.to_string();
                let bytes = match field.bytes().await {
                    Ok(b) => b.into_iter().collect::<Vec<_>>(),
                    Err(e) => return Err(e.body_text()),
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
                    .to_string();
                info_span!("Got directory name", dir);
                directory = Some(dir);
            } else {
                warn_span!("Unknown field", ?field);
            }
        }

        match directory {
            Some(directory) => Ok(FileUpload { directory, files }),
            None => Err("Directory field cannot be empty".to_string()),
        }
    }
}

#[derive(Deserialize, Serialize, Debug, ToSchema)]
pub struct TreeDirectory {
    id: String,
    name: String,
    files: Vec<TreeFile>,
    // Manually specify the schema to break infinite recursion
    #[schema(schema_with = recursive_directory_schema)]
    directories: Vec<TreeDirectory>,
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

#[derive(Deserialize, Serialize, Debug, ToSchema)]
pub struct TreeFile {
    id: String,
    name: String,
    size: usize,
    date: String,
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
            id: if value.path == "/" {
                "/".to_string()
            } else {
                format!("{}/", value.path.clone())
            },
            name: value.name.clone(),
            files: {
                let mut files = value.files.values().map(TreeFile::from).collect::<Vec<_>>();
                files.sort_by(|left, right| left.name.cmp(&right.name));
                files
            },
            directories: {
                let mut directories = value
                    .children
                    .values()
                    .map(TreeDirectory::from)
                    .collect::<Vec<_>>();
                directories.sort_by(|left, right| left.name.cmp(&right.name));
                directories
            },
        }
    }
}

#[derive(Serialize, Debug, ToSchema)]
pub struct ListView(Vec<ListViewItem>);

impl From<&Directory> for ListView {
    fn from(value: &Directory) -> Self {
        let mut children = Vec::new();
        let mut visit_dirs = VecDeque::from([value]);
        while let Some(directory) = visit_dirs.pop_front() {
            let mut directories = directory
                .children
                .values()
                .map(ListViewItem::from)
                .collect::<Vec<_>>();
            directories.sort_by(|left, right| left.id.cmp(&right.id));
            children.append(&mut directories);
            visit_dirs.extend(directory.children.values());

            let mut files = directory
                .files
                .values()
                .map(ListViewItem::from)
                .collect::<Vec<_>>();
            files.sort_by(|left, right| left.id.cmp(&right.id));
            children.append(&mut files);
        }
        ListView(children)
    }
}

#[derive(Serialize, Deserialize, Debug, ToSchema)]
pub struct ListViewItem {
    id: String,
    size: usize,
    date: String,
    r#type: ListViewItemType,
}

#[derive(Serialize, Deserialize, Debug, ToSchema)]
enum ListViewItemType {
    #[serde(rename = "folder")]
    Directory,
    #[serde(rename = "file")]
    File,
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

#[derive(Serialize, Deserialize, Debug, ToSchema)]
pub struct RenameRequest {
    ids_from: Vec<String>,
    ids_to: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, ToSchema)]
pub struct DeleteFilesRequest {
    /// path of files /dirs to be deleted.
    ///
    /// May not handle case where a folder and a file inside the folder is to be deleted in the same request
    ids: Vec<String>,
}

pub type Response<T> = Result<Json<T>, (StatusCode, String)>;

fn file_server_error(error: FileServerError) -> (StatusCode, String) {
    match error {
        FileServerError::Conflict => (StatusCode::CONFLICT, error.to_string()),
        FileServerError::TransactionClosed
        | FileServerError::Redis(_)
        | FileServerError::InvalidData(_) => (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()),
    }
}

const MAX_FILE_TRANSACTION_ATTEMPTS: usize = 3;

async fn retry_file_transaction<T, F>(
    file_server: &FileServer,
    mut apply: F,
) -> Result<Result<T, String>, FileServerError>
where
    F: for<'a> FnMut(&'a mut FileTransaction) -> BoxFuture<'a, Result<T, String>>,
{
    for attempt in 0..MAX_FILE_TRANSACTION_ATTEMPTS {
        let mut transaction = file_server.begin_transaction().await?;
        let result = apply(&mut transaction).await;
        let value = match result {
            Ok(value) => value,
            Err(message) => {
                transaction.abort_transaction().await;
                return Ok(Err(message));
            }
        };

        match transaction.write().await {
            Ok(()) => return Ok(Ok(value)),
            Err(FileServerError::Conflict) if attempt + 1 < MAX_FILE_TRANSACTION_ATTEMPTS => {
                drop(transaction);
                tokio::time::sleep(Duration::from_millis(5 * (attempt as u64 + 1))).await;
            }
            Err(error) => return Err(error),
        }
    }

    unreachable!("transaction attempts always return or continue")
}

#[utoipa::path(
    post,
    path = "/",
    tag = "files",
    request_body(content = FileUpload, content_type = "multipart/form-data"),
    responses(
        (status = 200, description = "Files uploaded successfully", body = ListView),
        (status = 400, description = "Bad Request (e.g. missing directory field)", body = String)
    )
)]
#[debug_handler]
pub async fn add_files(State(state): State<AppState>, multipart: Multipart) -> Response<ListView> {
    let upload = match FileUpload::from_multipart(multipart).await {
        Ok(u) => u,
        Err(message) => {
            return Err((StatusCode::BAD_REQUEST, message));
        }
    };
    let directory = upload.directory;
    let files = Arc::new(upload.files);
    let errors = retry_file_transaction(&state.file_server, |transaction| {
        let directory = directory.clone();
        let files = Arc::clone(&files);
        Box::pin(async move {
            if files.is_empty() {
                info_span!("No files in request; creating dirs");
                transaction
                    .add_dir(&directory)
                    .map_err(|message| format!("{message} ({directory})"))?;
                return Ok(Vec::new());
            }

            let mut errors = Vec::new();
            for file_item in files.iter() {
                if let Err(message) = transaction
                    .add_file(
                        &format!("{directory}/{}", file_item.name),
                        &file_item.content,
                    )
                    .await
                {
                    errors.push(message);
                }
            }
            Ok(errors)
        })
    })
    .await
    .map_err(file_server_error)?
    .map_err(|message| (StatusCode::BAD_REQUEST, message))?;

    if errors.is_empty() {
        Ok(Json(ListView(vec![])))
    } else {
        Err((StatusCode::BAD_REQUEST, errors.join(", ")))
    }
}

#[utoipa::path(
    get,
    path = "/list",
    tag = "files",
    responses(
        (status = 200, description = "List all files flat", body = ListView)
    )
)]
pub async fn get_all_paths_list(State(state): State<AppState>) -> Response<ListView> {
    let root = state
        .file_server
        .read_tree()
        .await
        .map_err(file_server_error)?;
    let files = (&root).into();
    Ok(Json(files))
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
    let root = state
        .file_server
        .read_tree()
        .await
        .map_err(file_server_error)?;
    let files = (&root).into();
    Ok(Json(files))
}

pub async fn get_file(State(state): State<AppState>, uri: Uri) -> impl IntoResponse {
    let url_decoded_path = urlencoding::decode(&uri.to_string()).unwrap().into_owned();
    let path = match state.file_server.get_file(&url_decoded_path).await {
        Ok(path) => path,
        Err(error) => return Err(file_server_error(error)),
    };

    match path {
        Some(p) => {
            let req = Request::builder()
                .uri(uri.clone())
                .body(Body::empty())
                .unwrap();
            let f = ServeFile::new(state.file_server.path.join(&p));
            f.oneshot(req)
                .await
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")))
        }
        None => Err((StatusCode::NOT_FOUND, format!("{uri} not found"))),
    }
}

/// Move/Rename files and folders
///
/// Multiple files and folders can be renamed at once. The from and to paths should be on the corresponding index of the `ids_from`
/// and the `ids_to` respectively.
///
/// ids ending with a `'/'` will be treated as a dir, and recursively move all contained items if present.
#[utoipa::path(
    put,
    path = "/",
    tag = "files",
    request_body = RenameRequest,
    responses(
        (status = 200, description = "Files and folders renamed successfully", body = ListView),
        (status = 400, description = "Bad Request", body = String)
    )
)]
#[debug_handler]
pub async fn rename_files(
    State(state): State<AppState>,
    Json(files): Json<RenameRequest>,
) -> Response<ListView> {
    info_span!("Renaming files", ?files);
    if files.ids_from.len() != files.ids_to.len() {
        return Err((
            StatusCode::BAD_REQUEST,
            "ids_from and ids_to must be the same length".to_string(),
        ));
    }
    let renames = files
        .ids_from
        .into_iter()
        .zip(files.ids_to)
        .collect::<Vec<_>>();
    let errors = retry_file_transaction(&state.file_server, |transaction| {
        let renames = renames.clone();
        Box::pin(async move {
            let mut errors = Vec::new();
            for (from, to) in renames {
                if from.ends_with('/') && to.ends_with('/') {
                    if let Err(message) = transaction.move_dir(&from, &to) {
                        errors.push(message);
                    }
                } else if !from.ends_with('/') && !to.ends_with('/') {
                    if let Err(message) = transaction.move_file(&from, &to) {
                        errors.push(message);
                    }
                } else {
                    errors.push(
                        "Cannot mix file and directories on the corresponding indexes of the arrays fields"
                            .to_string(),
                    )
                }
            }
            Ok(errors)
        })
    })
    .await
    .map_err(file_server_error)?
    .map_err(|message| (StatusCode::BAD_REQUEST, message))?;

    if errors.is_empty() {
        Ok(Json(ListView(vec![])))
    } else {
        Err((StatusCode::BAD_REQUEST, errors.join(", ")))
    }
}

/// Delete files and directories
///
/// ids ending with a `'/'` will be treated as a dir, and recursively remove all contained items if present.
#[utoipa::path(
    delete,
    path = "/",
    tag = "files",
    request_body = DeleteFilesRequest,
    responses(
        (status = 200, description = "Files deleted successfully", body = ListView),
        (status = 400, description = "Bad Request", body = String)
    )
)]
#[debug_handler]
pub async fn delete_files(
    State(state): State<AppState>,
    Json(files): Json<DeleteFilesRequest>,
) -> Response<ListView> {
    info_span!("Deleting files", ?files);
    let ids = files.ids;
    let errors = retry_file_transaction(&state.file_server, |transaction| {
        let ids = ids.clone();
        Box::pin(async move {
            let mut errors = Vec::new();
            for id in ids {
                if id.ends_with('/') {
                    if let Err(message) = transaction.delete_dir(&id) {
                        errors.push(message);
                    }
                } else if let Err(message) = transaction.delete_file(&id) {
                    errors.push(message);
                }
            }
            Ok(errors)
        })
    })
    .await
    .map_err(file_server_error)?
    .map_err(|message| (StatusCode::BAD_REQUEST, message))?;

    if errors.is_empty() {
        Ok(Json(ListView(vec![])))
    } else {
        Err((StatusCode::BAD_REQUEST, errors.join(", ")))
    }
}

pub fn file_api_router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(get_all_paths_tree))
        .routes(routes!(get_all_paths_list))
        .routes(routes!(delete_files))
        .routes(routes!(rename_files))
        .routes(routes!(add_files))
        .layer(DefaultBodyLimit::max(100_000_000))
}

#[cfg(test)]
mod tests {
    use crate::{
        file_server::file_server::{
            DIR_REGEX, FILE_PATH_REGEX, FileServer, FileServerError, VirtualPath,
        },
        routes::test_support::{app_state, cleanup_test_file_metadata},
    };

    use super::*;
    use axum::serve;
    use reqwest::multipart;
    use tokio::{fs, net::TcpListener};

    async fn peer_app_state(state: &AppState) -> AppState {
        let redis_url = std::env::var("REDIS_URL").expect("REDIS_URL must be set for tests");
        let file_server = FileServer::new_with_key(
            &redis_url,
            state.file_server.path.clone(),
            state.file_server.metadata_key().to_string(),
        )
        .await;
        AppState {
            store: state.store.clone(),
            file_server,
            events: state.events.clone(),
            htmx_hash: state.htmx_hash.clone(),
        }
    }

    pub fn get_file_in(root: &Directory, file_path: &str) -> Option<String> {
        let path = VirtualPath::parse_file(file_path).ok()?;
        let mut directory = root;
        for parent in path.parent() {
            directory = directory.children.get(*parent)?;
        }
        directory
            .files
            .get(path.name())
            .map(|file| file.file_server.clone())
    }

    #[tokio::test]
    async fn test_regex_filepath_validation() {
        // Valid paths
        assert!(FILE_PATH_REGEX.is_match("/file.txt"));
        assert!(FILE_PATH_REGEX.is_match("/folder/file.txt"));
        assert!(FILE_PATH_REGEX.is_match("/folder 1/my_filename.txt"));
        assert!(FILE_PATH_REGEX.is_match("/deeply/nested/dir/file.ts"));
        assert!(FILE_PATH_REGEX.is_match("/file_name.ts"));
        assert!(FILE_PATH_REGEX.is_match("/test-file.ts"));

        // Invalid paths
        assert!(!FILE_PATH_REGEX.is_match("test/file.txt")); // Missing leading slash
        assert!(!FILE_PATH_REGEX.is_match("file.txt"));
        assert!(!FILE_PATH_REGEX.is_match("/deeply/nested/dir/file.d.ts")); // only one extension
        assert!(!FILE_PATH_REGEX.is_match("/folder/")); // Ends in slash (not a file)
        assert!(!FILE_PATH_REGEX.is_match("/fol@der/file.txt")); // Illegal characters
    }

    #[tokio::test]
    async fn test_regex_dir_path_validation() {
        // Valid paths
        assert!(DIR_REGEX.is_match("/"));
        assert!(DIR_REGEX.is_match("/test/"));
        assert!(DIR_REGEX.is_match("/folder/test/"));
        assert!(DIR_REGEX.is_match("/folder 1/_test/"));
        assert!(DIR_REGEX.is_match("/deeply/nested/dir/here/"));

        // Invalid paths
        assert!(!DIR_REGEX.is_match("test/")); // Missing leading slash
        assert!(!DIR_REGEX.is_match("/test/tes")); // Missing ending slash
        assert!(!DIR_REGEX.is_match("/test/file.txt")); // File not dir
        assert!(!DIR_REGEX.is_match("/folder/.hidden/")); // No dots
        assert!(!DIR_REGEX.is_match("/fol@der/my_secret/")); // Illegal characters
        assert!(!DIR_REGEX.is_match("/../wonky/")); // Illegal characters
    }

    #[tokio::test]
    async fn test_path_of_root() {
        let server = app_state().await;
        let root = server.file_server.read_tree().await.unwrap();
        let tree: TreeDirectory = (&root).into();
        assert_eq!(tree.id, "/");
    }

    #[tokio::test]
    async fn test_metadata_uses_object_maps_and_commits_are_visible_to_new_requests() {
        let server = app_state().await;
        let mut transaction = server.file_server.begin_transaction().await.unwrap();
        let file = transaction
            .add_file("/nested/file.txt", b"contents")
            .await
            .unwrap();

        transaction.write().await.unwrap();
        let reloaded = server.file_server.read_tree().await.unwrap();
        let document = serde_json::to_value(&reloaded).unwrap();
        assert!(document["files"].is_object());
        assert!(document["children"].is_object());
        assert!(document["children"]["nested"]["files"].is_object());
        assert!(document["children"]["nested"]["files"]["file.txt"].is_object());
        assert_eq!(
            get_file_in(&reloaded, "/nested/file.txt"),
            Some(file.file_server)
        );
    }

    #[tokio::test]
    async fn test_stale_transaction_conflicts_and_removes_uncommitted_upload() {
        let server = app_state().await;
        let mut transaction = server.file_server.begin_transaction().await.unwrap();
        let mut stale = server.file_server.begin_transaction().await.unwrap();

        transaction.add_file("/committed.txt", &[1]).await.unwrap();
        transaction.write().await.unwrap();

        let stale_file = stale.add_file("/stale.txt", &[2]).await.unwrap();
        let stale_disk_path = server.file_server.path.join(&stale_file.file_server);
        assert!(stale_disk_path.exists());
        assert!(matches!(
            stale.write().await,
            Err(crate::file_server::file_server::FileServerError::Conflict)
        ));
        assert!(!stale_disk_path.exists());

        let reloaded = server.file_server.read_tree().await.unwrap();
        assert!(get_file_in(&reloaded, "/committed.txt").is_some());
        assert!(get_file_in(&reloaded, "/stale.txt").is_none());
    }

    #[tokio::test]
    async fn test_add_file_creates_directories() {
        let server = app_state().await;
        let mut transaction = server.file_server.begin_transaction().await.unwrap();

        let path = "/test_folder/nested/file.txt";
        let file = transaction
            .add_file(path, &[0x00; 1024])
            .await
            .expect("Failed to add file");

        assert_eq!(file.name, "file.txt");
        assert_eq!(file.path, path);
        assert_eq!(file.size, 1024);

        let no_file_ext = transaction.add_file("/test", &[]).await;
        assert_eq!(
            no_file_ext.unwrap_err(),
            "Illegal file name '/test'".to_string()
        );

        transaction.write().await.unwrap();
        let root = server.file_server.read_tree().await.unwrap();
        let tree: TreeDirectory = (&root).into();

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

        assert_eq!(test_folder.id, "/test_folder/");
        assert_eq!(nested.id, "/test_folder/nested/");
        assert_eq!(nested.files.len(), 1);
        assert_eq!(nested.files[0].name, "file.txt");
        assert_eq!(nested.files[0].id, "/test_folder/nested/file.txt");
    }

    #[tokio::test]
    async fn test_add_duplicate_file_fails() {
        let server = app_state().await;
        let mut transaction = server.file_server.begin_transaction().await.unwrap();

        let path = "/duplicates/file.txt";

        let _ = transaction.add_file(path, &[]).await.unwrap();

        let result = transaction.add_file(path, &[]).await;
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            "File /duplicates/file.txt already exists"
        );
    }

    #[tokio::test]
    async fn test_delete_file_success_and_failure() {
        let server = app_state().await;
        let mut transaction = server.file_server.begin_transaction().await.unwrap();
        let path = "/to_delete/delete_me.txt";

        let file = transaction.add_file(path, &[]).await.unwrap();

        let disk_path = server.file_server.path.join(&file.file_server);
        assert!(disk_path.exists());

        let deleted = transaction
            .delete_file(&path)
            .expect("Failed to delete file");
        assert_eq!(deleted.name, "delete_me.txt");

        assert!(disk_path.exists(), "deletion is staged until commit");
        transaction.write().await.unwrap();
        assert!(!disk_path.exists());

        let mut followup = server.file_server.begin_transaction().await.unwrap();
        let fail_result = followup.delete_file("/to_delete/does_not_exist.txt");
        assert!(fail_result.is_err());
        assert_eq!(
            fail_result.unwrap_err(),
            "File /to_delete/does_not_exist.txt does not exists"
        );
    }

    #[tokio::test]
    async fn test_add_and_delete_directory() {
        let server = app_state().await;
        let mut transaction = server.file_server.begin_transaction().await.unwrap();

        let dir_path = "/my_folder/child_folder/";
        transaction.add_dir(dir_path).expect("Failed to add dir");

        let file = transaction
            .add_file("/my_folder/child_folder/test.txt", &[])
            .await
            .unwrap();
        let disk_path = server.file_server.path.join(&file.file_server);
        assert!(disk_path.exists());

        let deleted_dir = transaction
            .delete_dir("/my_folder/")
            .expect("Failed to delete dir");
        assert_eq!(deleted_dir.name, "my_folder");

        assert!(disk_path.exists(), "deletion is staged until commit");
        transaction.write().await.unwrap();
        assert!(!disk_path.exists());

        let root = server.file_server.read_tree().await.unwrap();
        let tree: TreeDirectory = (&root).into();
        assert!(tree.directories.iter().all(|d| d.name != "my_folder"));
    }

    #[tokio::test]
    async fn test_delete_root_directory_fails() {
        let server = app_state().await;
        let mut transaction = server.file_server.begin_transaction().await.unwrap();

        // Attempting to delete "/" should be blocked
        let result = transaction.delete_dir("/");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Will not delete root folder");
    }

    #[tokio::test]
    async fn test_virtual_path_weirdness_get_normalized() {
        let server = app_state().await;
        let mut transaction = server.file_server.begin_transaction().await.unwrap();

        let weird_path = "///weird////path//file.txt";

        let file = transaction.add_file(weird_path, &[]).await.unwrap();
        assert_eq!(file.path, "/weird/path/file.txt");

        let weird_path = "/../wonky/.file/....path..txt";
        let file = transaction.add_file(weird_path, &[]).await;
        assert_eq!(
            file.unwrap_err(),
            format!("Illegal file name '{weird_path}'")
        );

        transaction.write().await.unwrap();
        let root = server.file_server.read_tree().await.unwrap();
        let tree: TreeDirectory = (&root).into();
        let weird_dir = tree.directories.iter().find(|d| d.name == "weird").unwrap();
        let path_dir = weird_dir
            .directories
            .iter()
            .find(|d| d.name == "path")
            .unwrap();

        assert_eq!(path_dir.files[0].name, "file.txt");
    }

    #[tokio::test]
    async fn test_move_file_cross_directory_and_rename() {
        let server = app_state().await;
        let mut transaction = server.file_server.begin_transaction().await.unwrap();
        transaction.add_dir("/folder_a/").unwrap();
        transaction.add_dir("/folder_b/").unwrap();
        transaction
            .add_file("/folder_a/test.txt", &[])
            .await
            .unwrap();

        // Move and rename at the same time
        let moved_file = transaction
            .move_file("/folder_a/test.txt", "/folder_b/moved.txt")
            .expect("Failed to move file");

        assert_eq!(moved_file.path, "/folder_b/moved.txt");
        assert_eq!(moved_file.name, "moved.txt");

        transaction.write().await.unwrap();
        let root = server.file_server.read_tree().await.unwrap();
        let tree: TreeDirectory = (&root).into();
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
        let server = app_state().await;
        let mut transaction = server.file_server.begin_transaction().await.unwrap();
        transaction.add_dir("/docs/").unwrap();
        transaction.add_file("/docs/apple.txt", &[]).await.unwrap();
        transaction.add_file("/docs/banana.txt", &[]).await.unwrap();
        transaction.add_file("/docs/zebra.txt", &[]).await.unwrap();

        // Rename apple to carrot. It must be inserted between banana and zebra.
        // If the index shift bug isn't fixed, it will break the alphabetical order.
        transaction
            .move_file("/docs/apple.txt", "/docs/carrot.txt")
            .unwrap();

        transaction.write().await.unwrap();
        let root = server.file_server.read_tree().await.unwrap();
        let tree: TreeDirectory = (&root).into();
        let docs = tree.directories.iter().find(|d| d.name == "docs").unwrap();

        assert_eq!(docs.files.len(), 3);
        assert_eq!(docs.files[0].name, "banana.txt");
        assert_eq!(docs.files[1].name, "carrot.txt");
        assert_eq!(docs.files[2].name, "zebra.txt");
    }

    #[tokio::test]
    async fn test_move_file_collisions_and_errors() {
        let server = app_state().await;
        let mut transaction = server.file_server.begin_transaction().await.unwrap();
        transaction.add_file("/docs/file1.txt", &[]).await.unwrap();
        transaction.add_file("/docs/file2.txt", &[]).await.unwrap();
        transaction
            .add_file("/archive/file1.txt", &[])
            .await
            .unwrap();

        // 1. Same directory collision
        let err1 = transaction
            .move_file("/docs/file1.txt", "/docs/file2.txt")
            .unwrap_err();
        assert!(err1.contains("already exists"));

        // 2. Cross directory collision
        let err2 = transaction
            .move_file("/docs/file1.txt", "/archive/file1.txt")
            .unwrap_err();
        assert!(err2.contains("already exists"));

        // 3. Source does not exist
        let err3 = transaction
            .move_file("/docs/ghost.txt", "/archive/ghost.txt")
            .unwrap_err();
        assert!(err3.contains("does not exist"));
    }

    #[tokio::test]
    async fn test_move_dir_recursive_path_updates() {
        let server = app_state().await;
        let mut transaction = server.file_server.begin_transaction().await.unwrap();
        transaction
            .add_file("/parent/child/deep/file.txt", &[])
            .await
            .unwrap();
        transaction.add_dir("/archive/").unwrap();

        let moved_dir = transaction
            .move_dir("/parent/child/", "/archive/renamed_child/")
            .unwrap();

        assert_eq!(moved_dir.path, "/archive/renamed_child");

        transaction.write().await.unwrap();
        let root = server.file_server.read_tree().await.unwrap();
        let tree: TreeDirectory = (&root).into();
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
        let server = app_state().await;
        let mut transaction = server.file_server.begin_transaction().await.unwrap();
        transaction.add_dir("/docs/archive/").unwrap();

        // 1. Block moving into itself
        let err1 = transaction.move_dir("/docs/", "/docs/").unwrap_err();
        assert!(err1.contains("Cannot move a directory into itself"));

        // 2. Block moving into its own child (Orphan Tree Bug)
        let err2 = transaction
            .move_dir("/docs/", "/docs/archive/nested/")
            .unwrap_err();
        assert!(err2.contains("Cannot move a directory into itself"));

        // 3. DO NOT block moving into a different folder with a similar prefix name!
        // This ensures new_dir_path.starts_with(&format!("{}/", old_dir_path)) is working perfectly.
        transaction.add_dir("/docs_new/").unwrap();
        let success = transaction.move_dir("/docs/", "/docs_new/docs/");

        assert!(
            success.is_ok(),
            "Failed to move into similarly named sibling directory"
        );
    }

    #[tokio::test]
    async fn test_full_api_upload_read_delete() {
        let state = app_state().await;
        let storage_path = state.file_server.path.clone();

        let (app, _api) = file_api_router()
            .with_state(state.clone())
            .split_for_parts();

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let base_url = format!("http://{}", addr);

        let server_task = tokio::spawn(async move {
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
        server_task.abort();
        fs::remove_dir_all(storage_path).await.unwrap();
        cleanup_test_file_metadata(&state).await.unwrap();
    }

    #[tokio::test]
    async fn independent_file_server_instances_share_metadata_and_bytes() {
        let server = app_state().await;
        let second = peer_app_state(&server).await;
        let path = "/shared/file.txt";

        let mut upload = server.file_server.begin_transaction().await.unwrap();
        let file = upload.add_file(path, b"shared bytes").await.unwrap();
        upload.write().await.unwrap();

        let second_file_name = second.file_server.get_file(&path).await.unwrap().unwrap();
        assert_eq!(second_file_name, file.file_server);
        assert_eq!(
            fs::read(second.file_server.path.join(&second_file_name))
                .await
                .unwrap(),
            b"shared bytes"
        );

        let mut delete = second.file_server.begin_transaction().await.unwrap();
        delete.delete_file(&path).unwrap();
        delete.write().await.unwrap();

        assert!(server.file_server.get_file(&path).await.unwrap().is_none());
        assert!(!server.file_server.path.join(file.file_server).exists());
    }

    #[tokio::test]
    async fn simultaneous_disjoint_writes_conflict_then_retry_without_lost_updates() {
        let server = app_state().await;
        let second = peer_app_state(&server).await;
        let left_path = "/parallel/left.txt";
        let right_path = "/parallel/right.txt";

        let mut left = server.file_server.begin_transaction().await.unwrap();
        let mut right = second.file_server.begin_transaction().await.unwrap();
        let left_file = left.add_file(left_path, b"left").await.unwrap();
        let right_file = right.add_file(right_path, b"right").await.unwrap();

        let (left_result, right_result) = tokio::join!(left.write(), right.write());
        match (&left_result, &right_result) {
            (Ok(()), Err(FileServerError::Conflict)) | (Err(FileServerError::Conflict), Ok(())) => {
            }
            results => panic!("expected one commit and one conflict, got {results:?}"),
        }

        let retry_path;
        let retry_content: &[u8];
        let rejected_file;
        if left_result.is_ok() {
            retry_path = right_path;
            retry_content = b"right";
            rejected_file = right_file;
            assert!(
                server
                    .file_server
                    .path
                    .join(&left_file.file_server)
                    .exists()
            );
        } else {
            retry_path = left_path;
            retry_content = b"left";
            rejected_file = left_file;
            assert!(
                server
                    .file_server
                    .path
                    .join(&right_file.file_server)
                    .exists()
            );
        }
        assert!(
            !server
                .file_server
                .path
                .join(&rejected_file.file_server)
                .exists()
        );

        let mut retry = second.file_server.begin_transaction().await.unwrap();
        retry.add_file(retry_path, retry_content).await.unwrap();
        retry.write().await.unwrap();

        assert!(
            second
                .file_server
                .get_file(&left_path)
                .await
                .unwrap()
                .is_some()
        );
        assert!(
            second
                .file_server
                .get_file(&right_path)
                .await
                .unwrap()
                .is_some()
        );
        for (path, expected) in [
            (&left_path, b"left".as_slice()),
            (&right_path, b"right".as_slice()),
        ] {
            let filename = second.file_server.get_file(path).await.unwrap().unwrap();
            assert_eq!(
                fs::read(second.file_server.path.join(filename))
                    .await
                    .unwrap(),
                expected
            );
        }
    }

    #[tokio::test]
    async fn retrying_disjoint_file_operations_commits_both_requests() {
        use std::sync::atomic::{AtomicUsize, Ordering};

        let server = app_state().await;
        let second = peer_app_state(&server).await;
        let barrier = Arc::new(tokio::sync::Barrier::new(2));
        let left_attempts = Arc::new(AtomicUsize::new(0));
        let right_attempts = Arc::new(AtomicUsize::new(0));

        let left_result = retry_file_transaction(&server.file_server, {
            let barrier = Arc::clone(&barrier);
            let attempts = Arc::clone(&left_attempts);
            move |transaction| {
                let barrier = Arc::clone(&barrier);
                let attempt = attempts.fetch_add(1, Ordering::SeqCst);
                Box::pin(async move {
                    transaction.add_file("/retry-left.txt", b"left").await?;
                    if attempt == 0 {
                        barrier.wait().await;
                    }
                    Ok(())
                })
            }
        });
        let right_result = retry_file_transaction(&second.file_server, {
            let barrier = Arc::clone(&barrier);
            let attempts = Arc::clone(&right_attempts);
            move |transaction| {
                let barrier = Arc::clone(&barrier);
                let attempt = attempts.fetch_add(1, Ordering::SeqCst);
                Box::pin(async move {
                    transaction.add_file("/retry-right.txt", b"right").await?;
                    if attempt == 0 {
                        barrier.wait().await;
                    }
                    Ok(())
                })
            }
        });

        let (left_result, right_result) = tokio::join!(left_result, right_result);
        assert!(matches!(left_result, Ok(Ok(()))), "{left_result:?}");
        assert!(matches!(right_result, Ok(Ok(()))), "{right_result:?}");
        assert_eq!(
            left_attempts.load(Ordering::SeqCst) + right_attempts.load(Ordering::SeqCst),
            3
        );
        assert!(
            server
                .file_server
                .get_file("/retry-left.txt")
                .await
                .unwrap()
                .is_some()
        );
        assert!(
            server
                .file_server
                .get_file("/retry-right.txt")
                .await
                .unwrap()
                .is_some()
        );
    }

    #[tokio::test]
    async fn retry_revalidates_same_path_and_returns_duplicate_error() {
        use std::sync::atomic::{AtomicUsize, Ordering};

        let server = app_state().await;
        let second = peer_app_state(&server).await;
        let barrier = Arc::new(tokio::sync::Barrier::new(2));
        let left_attempts = Arc::new(AtomicUsize::new(0));
        let right_attempts = Arc::new(AtomicUsize::new(0));
        let path = "/retry-same.txt";

        let left_result = retry_file_transaction(&server.file_server, {
            let barrier = Arc::clone(&barrier);
            let attempts = Arc::clone(&left_attempts);
            move |transaction| {
                let barrier = Arc::clone(&barrier);
                let attempt = attempts.fetch_add(1, Ordering::SeqCst);
                Box::pin(async move {
                    let file = transaction.add_file(path, b"left").await?;
                    if attempt == 0 {
                        barrier.wait().await;
                    }
                    Ok(file)
                })
            }
        });
        let right_result = retry_file_transaction(&second.file_server, {
            let barrier = Arc::clone(&barrier);
            let attempts = Arc::clone(&right_attempts);
            let path = path;
            move |transaction| {
                let barrier = Arc::clone(&barrier);
                let path = path;
                let attempt = attempts.fetch_add(1, Ordering::SeqCst);
                Box::pin(async move {
                    let file = transaction.add_file(path, b"right").await?;
                    if attempt == 0 {
                        barrier.wait().await;
                    }
                    Ok(file)
                })
            }
        });

        let (left_result, right_result) = tokio::join!(left_result, right_result);
        match (&left_result, &right_result) {
            (Ok(Ok(_)), Ok(Err(message))) | (Ok(Err(message)), Ok(Ok(_))) => {
                assert!(message.contains("already exists"), "{message}");
            }
            results => panic!("expected one success and one duplicate error, got {results:?}"),
        }
        assert_eq!(
            left_attempts.load(Ordering::SeqCst) + right_attempts.load(Ordering::SeqCst),
            3
        );
        assert!(server.file_server.get_file(&path).await.unwrap().is_some());
    }

    #[tokio::test]
    async fn simultaneous_uploads_to_same_path_commit_only_one_file() {
        let server = app_state().await;
        let second = peer_app_state(&server).await;
        let path = "/same-name.txt";

        let mut first = server.file_server.begin_transaction().await.unwrap();
        let mut second_tx = second.file_server.begin_transaction().await.unwrap();
        let first_file = first.add_file(path, b"first").await.unwrap();
        let second_file = second_tx.add_file(path, b"second").await.unwrap();

        let (first_result, second_result) = tokio::join!(first.write(), second_tx.write());
        match (&first_result, &second_result) {
            (Ok(()), Err(FileServerError::Conflict)) | (Err(FileServerError::Conflict), Ok(())) => {
            }
            results => panic!("expected one commit and one conflict, got {results:?}"),
        }

        let winner = server.file_server.get_file(&path).await.unwrap().unwrap();
        let winner_bytes = fs::read(server.file_server.path.join(&winner))
            .await
            .unwrap();
        let (loser, expected_winner_bytes) = if winner == first_file.file_server {
            (&second_file, b"first".as_slice())
        } else {
            (&first_file, b"second".as_slice())
        };
        assert_eq!(winner_bytes, expected_winner_bytes);
        assert!(!server.file_server.path.join(&loser.file_server).exists());
    }

    #[tokio::test]
    async fn write_twice_on_a_transaction_returns_transaction_closed() {
        let server = app_state().await;
        let mut transaction = server.file_server.begin_transaction().await.unwrap();
        transaction.add_file("/once.txt", b"once").await.unwrap();

        transaction.write().await.unwrap();
        assert!(matches!(
            transaction.write().await,
            Err(FileServerError::TransactionClosed)
        ));
    }

    #[tokio::test]
    async fn committed_transaction_cannot_replay_stale_metadata() {
        let server = app_state().await;
        let original_path = "/before.txt";
        let renamed_path = "/after.txt";

        let mut original = server.file_server.begin_transaction().await.unwrap();
        original.add_file(original_path, b"contents").await.unwrap();
        original.write().await.unwrap();

        let mut rename = server.file_server.begin_transaction().await.unwrap();
        rename.move_file(&original_path, &renamed_path).unwrap();
        rename.write().await.unwrap();

        assert!(matches!(
            original.write().await,
            Err(FileServerError::TransactionClosed)
        ));
        assert_eq!(
            None,
            server.file_server.get_file(&original_path).await.unwrap()
        );
        assert!(
            server
                .file_server
                .get_file(&renamed_path)
                .await
                .unwrap()
                .is_some()
        );
    }

    #[tokio::test]
    async fn conflicted_transaction_cannot_be_replayed_without_watch() {
        let server = app_state().await;
        let second = peer_app_state(&server).await;
        let stale_path = "/stale.txt";

        let mut stale = server.file_server.begin_transaction().await.unwrap();
        let stale_file = stale.add_file(stale_path, b"stale").await.unwrap();

        let mut winner = second.file_server.begin_transaction().await.unwrap();
        winner.add_file("/winner.txt", b"winner").await.unwrap();
        winner.write().await.unwrap();

        assert!(matches!(
            stale.write().await,
            Err(FileServerError::Conflict)
        ));
        assert!(
            !server
                .file_server
                .path
                .join(&stale_file.file_server)
                .exists()
        );

        assert!(matches!(
            stale.write().await,
            Err(FileServerError::TransactionClosed)
        ));
        assert!(
            server
                .file_server
                .get_file(&stale_path)
                .await
                .unwrap()
                .is_none()
        );
    }

    #[tokio::test]
    async fn aborted_transaction_cannot_overwrite_a_later_upload() {
        let server = app_state().await;
        let second = peer_app_state(&server).await;
        let mut aborted = server.file_server.begin_transaction().await.unwrap();
        aborted.add_file("/aborted.txt", b"discard").await.unwrap();
        aborted.abort_transaction().await;

        let raced_path = "/after-abort.txt";
        let post_abort = aborted.add_file(raced_path, b"post-abort").await;
        if let Err(message) = post_abort {
            assert!(
                message.to_lowercase().contains("closed")
                    || message.to_lowercase().contains("finished"),
                "unexpected error when reusing an aborted transaction: {message}"
            );
            return;
        }

        let mut winner = second.file_server.begin_transaction().await.unwrap();
        let winner_file = winner.add_file(raced_path, b"winner").await.unwrap();
        winner.write().await.unwrap();

        assert!(matches!(
            aborted.write().await,
            Err(FileServerError::TransactionClosed)
        ));
        assert_eq!(
            second
                .file_server
                .get_file(&raced_path)
                .await
                .unwrap()
                .as_deref(),
            Some(winner_file.file_server.as_str())
        );
    }

    #[tokio::test]
    async fn dropping_uncommitted_upload_does_not_leave_an_orphan_file() {
        let server = app_state().await;
        let path = "/abandoned.txt";
        let mut transaction = server.file_server.begin_transaction().await.unwrap();
        let file = transaction.add_file(path, b"abandoned").await.unwrap();
        let disk_path = server.file_server.path.join(file.file_server);
        drop(transaction);

        assert!(server.file_server.get_file(&path).await.unwrap().is_none());
        assert!(!disk_path.exists());
    }

    #[tokio::test]
    async fn delete_between_metadata_lookup_and_file_open_can_return_not_found() {
        let server = app_state().await;
        let second = peer_app_state(&server).await;
        let path = "/read-delete-race.txt";
        let mut upload = server.file_server.begin_transaction().await.unwrap();
        let file = upload.add_file(path, b"read me").await.unwrap();
        upload.write().await.unwrap();

        // This is the metadata lookup performed by the download handler.
        let resolved = server.file_server.get_file(&path).await.unwrap().unwrap();
        let mut delete = second.file_server.begin_transaction().await.unwrap();
        delete.delete_file(path).unwrap();
        delete.write().await.unwrap();

        // The handler opens the resolved path only after the metadata lookup.
        let response = ServeFile::new(server.file_server.path.join(resolved))
            .oneshot(
                Request::builder()
                    .uri("/files/read-delete-race.txt")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        assert!(!server.file_server.path.join(file.file_server).exists());
    }

    #[tokio::test]
    async fn concurrent_file_server_initialization_leaves_a_valid_root() {
        let server = app_state().await;
        let mut transaction = server.file_server.begin_transaction().await.unwrap();
        let redis_url = std::env::var("REDIS_URL").expect("REDIS_URL must be set for tests");
        let key = server.file_server.metadata_key().to_string();
        let path = server.file_server.path.clone();
        let client = redis::Client::open(redis_url.as_str()).unwrap();
        let mut con = client.get_multiplexed_async_connection().await.unwrap();
        redis::cmd("DEL")
            .arg(&key)
            .query_async::<()>(&mut con)
            .await
            .unwrap();
        drop(con);

        let (first, second) = tokio::join!(
            FileServer::new_with_key(&redis_url, path.clone(), key.clone()),
            FileServer::new_with_key(&redis_url, path, key),
        );

        assert_eq!(first.read_tree().await.unwrap().path, "/");
        assert_eq!(second.read_tree().await.unwrap().path, "/");
        transaction.abort_transaction().await;
    }

    #[tokio::test]
    async fn invalid_metadata_is_rejected_when_opening_a_transaction() {
        let server = app_state().await;
        let redis_url = std::env::var("REDIS_URL").expect("REDIS_URL must be set for tests");
        let client = redis::Client::open(redis_url).unwrap();
        let mut con = client.get_multiplexed_async_connection().await.unwrap();
        let _: Option<String> = redis::cmd("JSON.SET")
            .arg(server.file_server.metadata_key())
            .arg("$.files")
            .arg("\"not-an-object\"")
            .query_async(&mut con)
            .await
            .unwrap();
        drop(con);

        assert!(matches!(
            server.file_server.begin_transaction().await,
            Err(FileServerError::InvalidData(_))
        ));
    }
}
