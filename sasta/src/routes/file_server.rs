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
use tracing::{debug, error, instrument, warn};
use utoipa::{
    ToSchema,
    openapi::{ArrayBuilder, Ref, RefOr, Schema},
};
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::{
    AppState,
    file_server::file_server::{
        Directory, ErrorType, File, FileOperationError, FileServer, FileServerError,
        FileTransaction,
    },
};

/// Multipart form-data payload for uploading files or creating a directory.
///
/// ### Path Rules
/// - **Directories (`directory`)**: Must start and end with `/` (e.g. `"/"` or `"/documents/2026/"`), and may only contain alphanumeric characters, `_`, `-`, spaces, and `/`.
/// - **Filenames (`files`)**: Each uploaded file's name must end with a single extension (`<name>.<ext>`, e.g. `"poster.png"`) and contain only alphanumeric characters, `_`, `-`, and spaces.
/// - Maximum request body size is **100 MB** (`100_000_000` bytes).
#[derive(ToSchema)]
#[schema(example = json!({
    "directory": "/documents/2026/",
    "files": [
        {
            "name": "budget_q1.csv",
            "content": "Q2F0ZWdvcnksQW1vdW50ClNpdGUgSG9zdGluZywkMjUw"
        },
        {
            "name": "budget_statement_q1.txt",
            "content": "SXQncyBhIHNlY3JldCB0byBldmVyeWJvZHk="
        }
    ]
}))]
pub struct FileUpload {
    /// Target virtual directory path to upload files into (or create if `files` is empty).
    /// Must start and end with `/` (e.g. `"/documents/"`). Intermediate parent directories are created automatically.
    #[schema(example = "/documents/")]
    directory: String,
    /// One or more binary file parts (`multipart/form-data` field name `"files"`).
    /// If omitted or empty, Sasta creates the target `directory` as an empty folder.
    #[schema(value_type = Vec<String>, format = Binary)]
    files: Vec<FileItem>,
    #[serde(skip)]
    /// List of failed files (id, error_message)
    failed: Vec<(String, String)>,
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
        let mut failed = Vec::new();

        while let Some(field) = multipart.next_field().await.unwrap() {
            if let Some(filename) = field.file_name() {
                let filename = filename.to_string();
                let bytes = match field.bytes().await {
                    Ok(b) => b.into_iter().collect::<Vec<_>>(),
                    Err(e) => {
                        failed.push((filename, e.body_text()));
                        continue;
                    }
                };
                debug!("Add file {filename}");
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
                debug!("Got directory name {dir}");
                directory = Some(dir);
            } else {
                warn!("Unknown field {field:?}");
            }
        }

        match directory {
            Some(directory) => Ok(FileUpload {
                directory,
                files,
                failed,
            }),
            None => Err("Directory field cannot be empty".to_string()),
        }
    }
}

/// Recursive tree node representing a directory and its contents in the virtual filesystem.
#[derive(Serialize, Deserialize, Debug, ToSchema)]
#[schema(example = json!({
    "id": "/documents/",
    "name": "documents",
    "files": [
        {
            "id": "/documents/report.pdf",
            "name": "report.pdf",
            "size": 1048576,
            "date": "2026-10-05T12:00:00+00:00"
        }
    ],
    "directories": [
        {
            "id": "/documents/2026/",
            "name": "2026",
            "files": [
                {
                    "id": "/documents/2026/note.txt",
                    "name": "note.txt",
                    "size": 1067,
                    "date": "2026-10-05T12:00:00+00:00"
                }
            ],
            "directories": []
        }
    ]
}))]
pub struct TreeDirectory {
    /// Full virtual path of the directory, always starting and ending with `/` (e.g. `"/"` or `"/documents/"`).
    id: String,

    /// The local name of the directory (`""` for the root directory `"/"`).
    name: String,

    /// Alphabetically sorted list of files directly contained within this directory.
    files: Vec<TreeFile>,

    /// Alphabetically sorted list of nested child directories.
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

/// Metadata for a single file in the hierarchical tree view.
///
/// The file's raw contents can be downloaded or streamed at `GET /files{id}` (e.g. `GET /files/documents/report.pdf`).
#[derive(Serialize, Deserialize, Debug, ToSchema)]
pub struct TreeFile {
    /// Full virtual path to the file starting with `/` (e.g. `"/documents/report.pdf"`).
    #[schema(example = "/documents/report.pdf")]
    id: String,
    /// Filename with extension.
    #[schema(example = "report.pdf")]
    name: String,
    /// File size in bytes.
    #[schema(example = 1048576)]
    size: usize,
    /// Upload / last modified timestamp in RFC3339 format.
    #[schema(example = "2026-10-05T12:00:00+00:00")]
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

/// Flat list of all directories and files in breadth-first order.
#[derive(Serialize, Deserialize, Debug, ToSchema)]
#[schema(example = json!([
    {
        "id": "/documents",
        "size": 0,
        "date": "1996-12-19T16:39:57-08:00",
        "type": "folder"
    },
    {
        "id": "/documents/report.pdf",
        "size": 1048576,
        "date": "2026-10-05T12:05:00+00:00",
        "type": "file"
    }
]))]
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

/// A single file or folder entry in the flat `ListView`.
#[derive(Serialize, Deserialize, Debug, ToSchema)]
pub struct ListViewItem {
    /// Full virtual path of the item.
    #[schema(example = "/documents/report.pdf")]
    id: String,
    /// File size in bytes (`0` for folders).
    #[schema(example = 1048576)]
    size: usize,
    /// Last modified date in RFC3339 format (folders currently return a static placeholder timestamp).
    #[schema(example = "2026-10-05T12:05:00+00:00")]
    date: String,
    /// Whether this entry is a `"folder"` or `"file"`.
    r#type: ListViewItemType,
}

/// Discriminator for entries in `ListView`.
#[derive(Serialize, Deserialize, Debug, ToSchema)]
enum ListViewItemType {
    // TODO: Be consistent with naming and always use `directory`
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

/// Request payload for renaming or moving one or more files and directories.
///
/// `ids_from` and `ids_to` must have the exact same length. Each pair `(ids_from[i], ids_to[i])`
/// must either both be file paths (no trailing `/`) or both be directory paths (ending with `/`).
#[derive(Serialize, Deserialize, Debug, ToSchema)]
pub struct RenameRequest {
    /// Source virtual paths to rename or move. Directories must end with `/`; files must not.
    #[schema(example = json!(["/old_name.txt", "/old_folder/"]))]
    ids_from: Vec<String>,
    /// Destination virtual paths corresponding 1-to-1 with `ids_from`.
    /// Destination parent directories must already exist.
    #[schema(example = json!(["/new_name.txt", "/new_folder/"]))]
    ids_to: Vec<String>,
}

/// Request payload for deleting one or more files or directories.
#[derive(Serialize, Deserialize, Debug, ToSchema)]
pub struct DeleteFilesRequest {
    /// Virtual paths to delete. Paths ending with `/` are treated as directories and deleted recursively
    /// along with all their contents (except the root directory `"/"`, which cannot be deleted).
    #[schema(example = json!(["/folder_a/file.txt", "/folder_b/"]))]
    ids: Vec<String>,
}

/// Result summary for batch file upload (`POST /api/files`) and delete (`DELETE /api/files`) operations.
#[derive(Serialize, Deserialize, Debug, ToSchema)]
#[schema(example = json!({
    "successful": 1,
    "failed": 1,
    "statuses": [
        { "status": "success", "id": "/api_folder/file.txt" },
        { "status": "failure", "id": "/api_folder/duplicate_file.txt", "error_type": "alreadyExists", "error": "File already exists" }
    ]
}))]
pub struct BatchOperationResponse {
    /// Number of items that succeeded.
    pub successful: usize,
    /// Number of items that failed. If `failed > 0`, the HTTP response status is `207 Multi-Status`.
    pub failed: usize,
    /// Per-item status entries in the order processed.
    pub statuses: Vec<ItemStatus>,
}

/// Per-item outcome for an upload or delete operation.
#[derive(Serialize, Deserialize, Debug, ToSchema)]
#[serde(tag = "status", rename_all = "camelCase")]
pub enum ItemStatus {
    /// The file or directory operation succeeded.
    Success {
        /// Virtual path of the affected file or directory.
        id: String,
    },
    /// The file or directory operation failed.
    Failure {
        /// Virtual path of the file or directory that failed.
        id: String,
        /// Classification of the failure for client-side error handling.
        error_type: ErrorType,
        /// Human-readable reason for the failure.
        error: String,
    },
}

/// Result summary for batch rename/move (`PUT /api/files`) operations.
#[derive(Serialize, Deserialize, Debug, ToSchema)]
#[schema(example = json!({
    "successful": 1,
    "failed": 1,
    "statuses": [
        { "status": "success", "from": "/old_file.txt", "to": "/new_file.txt" },
        { "status": "failure", "from": "/missing.txt", "to": "/new.txt", "error_type": "notFound", "error": "File not found" }
    ]
}))]
pub struct RenameBatchOperationResponse {
    /// Number of items that were renamed/moved successfully.
    pub successful: usize,
    /// Number of items that failed. If `failed > 0`, the HTTP response status is `207 Multi-Status`.
    pub failed: usize,
    /// Per-item status entries for each `(from, to)` pair.
    pub statuses: Vec<RenameItemStatus>,
}

/// Per-item outcome for a rename/move operation.
#[derive(Serialize, Deserialize, Debug, ToSchema)]
#[serde(tag = "status", rename_all = "camelCase")]
pub enum RenameItemStatus {
    /// The item was renamed/moved successfully.
    Success {
        /// Original virtual path.
        from: String,
        /// New virtual path.
        to: String,
    },
    /// The item could not be renamed/moved.
    Failure {
        /// Original virtual path.
        from: String,
        /// Requested destination virtual path.
        to: String,
        /// Classification of the failure for client-side error handling.
        error_type: ErrorType,
        /// Human-readable reason for the failure.
        error: String,
    },
}

impl ItemStatus {
    pub fn is_success(&self) -> bool {
        matches!(self, ItemStatus::Success { .. })
    }
}

impl RenameItemStatus {
    pub fn is_success(&self) -> bool {
        matches!(self, RenameItemStatus::Success { .. })
    }
}

pub type Response<T> = Result<Json<T>, (StatusCode, String)>;
pub type BatchResponse<T> = Result<(StatusCode, Json<T>), (StatusCode, String)>;

fn file_server_error(error: FileServerError) -> (StatusCode, String) {
    match error {
        FileServerError::Conflict => {
            warn!("File server conflict: {error}");
            (StatusCode::CONFLICT, error.to_string())
        }
        FileServerError::TransactionClosed
        | FileServerError::Redis(_)
        | FileServerError::InvalidData(_) => {
            error!("File server error: {error}");
            (StatusCode::INTERNAL_SERVER_ERROR, error.to_string())
        }
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
    summary = "Upload files or create a directory",
    description = "Uploads one or more files via a `multipart/form-data` stream into a target `directory` (max body size **100 MB**).\n\n- If `directory` does not exist, it and any missing parent directories are created automatically.\n- If no `files` parts are provided, `directory` is created as an empty folder (in this mode `directory` **must** start and end with `/`, e.g. `\"/new_folder/\"`, and must not already exist).\n- Allowed path characters: `A-Z`, `a-z`, `0-9`, `_`, `-`, spaces, and `/`. File names must have a single extension (`<name>.<ext>`).\n- Per-file validation or collision errors are returned in the response body with HTTP status `207 Multi-Status` rather than failing the entire batch.",
    request_body(content = FileUpload, content_type = "multipart/form-data"),
    responses(
        (status = 200, description = "All files (or the directory) were created successfully", body = BatchOperationResponse),
        (status = 207, description = "Partial or per-item failure (e.g., file already exists, illegal filename, or directory already exists).", body = BatchOperationResponse),
        (status = 400, description = "Structural Bad Request (e.g., missing `directory` field in multipart form)", body = String),
        (status = 409, description = "Concurrent modification conflict after retries (`File metadata changed during this request; reload and retry`)", body = String),
        (status = 500, description = "Internal server / Redis error", body = String)
    )
)]
#[debug_handler]
#[instrument(level = "debug", skip(state, multipart))]
pub async fn add_files(
    State(state): State<AppState>,
    multipart: Multipart,
) -> BatchResponse<BatchOperationResponse> {
    let upload = match FileUpload::from_multipart(multipart).await {
        Ok(u) => u,
        Err(message) => {
            return Err((StatusCode::BAD_REQUEST, message));
        }
    };
    let directory = upload.directory;
    let files = Arc::new(upload.files);
    let mut statuses = retry_file_transaction(&state.file_server, |transaction| {
        let directory = directory.clone();
        let files = Arc::clone(&files);
        Box::pin(async move {
            if files.is_empty() {
                debug!("No files in request; creating dirs");
                match transaction.add_dir(&directory) {
                    Ok(_) => return Ok(vec![ItemStatus::Success { id: directory }]),
                    Err(err) => {
                        return Ok(vec![ItemStatus::Failure {
                            id: directory.clone(),
                            error_type: err.error_type,
                            error: format!("{} ({directory})", err.message),
                        }]);
                    }
                }
            }

            let mut statuses = Vec::new();
            for file_item in files.iter() {
                let id = format!("{}/{}", directory, file_item.name);
                match transaction.add_file(&id, &file_item.content).await {
                    Ok(_) => statuses.push(ItemStatus::Success { id }),
                    Err(err) => statuses.push(ItemStatus::Failure {
                        id,
                        error_type: err.error_type,
                        error: err.message,
                    }),
                }
            }
            Ok(statuses)
        })
    })
    .await
    .map_err(file_server_error)?
    .map_err(|message| (StatusCode::BAD_REQUEST, message))?;

    for (filename, error_msg) in upload.failed {
        statuses.push(ItemStatus::Failure {
            id: format!("{}/{}", directory, filename),
            error_type: ErrorType::UploadFailed,
            error: format!("Stream upload failed: {}", error_msg),
        });
    }

    let response = BatchOperationResponse {
        successful: statuses.iter().filter(|s| s.is_success()).count(),
        failed: statuses.iter().filter(|s| !s.is_success()).count(),
        statuses,
    };

    if response.failed > 0 {
        Ok((StatusCode::MULTI_STATUS, Json(response)))
    } else {
        Ok((StatusCode::OK, Json(response)))
    }
}

#[utoipa::path(
    get,
    path = "/list",
    tag = "files",
    summary = "Get a flat list of all files and folders",
    description = "Returns a flat, breadth-first list of all folders and files stored on the server. To download an individual file's contents, send a `GET` request to `/files{id}` (e.g. `GET /files/documents/report.pdf`).",
    responses(
        (status = 200, description = "A flat list of all files and directories", body = ListView),
        (status = 500, description = "Internal server / Redis error", body = String)
    )
)]
#[instrument(level = "debug", skip(state))]
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
    summary = "Get a hierarchical tree of all files and folders",
    description = "Returns a nested tree structure starting at the root directory (`\"/\"`) representing all directories and files on the server. Child directories and files at each level are sorted alphabetically by `name`. To download an individual file's contents, send a `GET` request to `/files{id}` (e.g. `GET /files/documents/report.pdf`).",
    responses(
        (status = 200, description = "A tree representation of the file system", body = TreeDirectory),
        (status = 500, description = "Internal server / Redis error", body = String)
    )
)]
#[instrument(level = "debug", skip(state))]
pub async fn get_all_paths_tree(State(state): State<AppState>) -> Response<TreeDirectory> {
    let root = state
        .file_server
        .read_tree()
        .await
        .map_err(file_server_error)?;
    let files = (&root).into();
    Ok(Json(files))
}

#[instrument(level = "debug", skip(state))]
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

#[utoipa::path(
    put,
    path = "/",
    tag = "files",
    summary = "Rename or move files and directories",
    description = "Renames or moves multiple files and directories in a single atomic transaction.\n\n**Rules:**\n- `ids_from` and `ids_to` must have equal length; otherwise `400 Bad Request` is returned.\n- At each index `i`, `ids_from[i]` and `ids_to[i]` must both be file paths (no trailing `/`) or both be directory paths (ending with `/`). Mixing a file and a directory at the same index returns `400 Bad Request`.\n- Destination parent directories must already exist. Per-item failures (such as a missing source path or existing destination path) are reported inside the `207 Multi-Status` response body.",
    request_body = RenameRequest,
    responses(
        (status = 200, description = "All items were renamed or moved successfully", body = RenameBatchOperationResponse),
        (status = 207, description = "Partial or per-item failure (e.g., source not found, destination already exists, or invalid path).", body = RenameBatchOperationResponse),
        (status = 400, description = "Bad Request (e.g., `ids_from` and `ids_to` length mismatch, or mixing file and directory paths at the same index)", body = String, examples(
            ("length_mismatch" = (
                summary = "ids_from and ids_to have different lengths",
                value = json!("ids_from and ids_to must be the same length")
            )),
            ("mixed_types" = (
                summary = "File path paired with directory path",
                value = json!("Cannot mix file and directories on the corresponding indexes of the arrays fields")
            ))
        )),
        (status = 409, description = "Concurrent modification conflict after retries (`File metadata changed during this request; reload and retry`)", body = String),
        (status = 500, description = "Internal server / Redis error", body = String)
    )
)]
#[debug_handler]
#[instrument(level = "debug", skip(state))]
pub async fn rename_files(
    State(state): State<AppState>,
    Json(files): Json<RenameRequest>,
) -> BatchResponse<RenameBatchOperationResponse> {
    debug!("Renaming files {files:?}");
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
    let statuses = retry_file_transaction(&state.file_server, |transaction| {
        let renames = renames.clone();
        Box::pin(async move {
            let mut statuses = Vec::new();
            for (from, to) in renames {
                let result = if from.ends_with('/') && to.ends_with('/') {
                    transaction.move_dir(&from, &to).map(|_| ())
                } else if !from.ends_with('/') && !to.ends_with('/') {
                    transaction.move_file(&from, &to).map(|_| ())
                } else {
                    Err(FileOperationError::new(
                        ErrorType::MixedFileAndDirectory,
                        "Cannot mix file and directories on the corresponding indexes of the arrays fields",
                    ))
                };

                match result {
                    Ok(_) => statuses.push(RenameItemStatus::Success { from, to }),
                    Err(err) => statuses.push(RenameItemStatus::Failure {
                        from,
                        to,
                        error_type: err.error_type,
                        error: err.message,
                    }),
                }
            }
            Ok(statuses)
        })
    })
    .await
    .map_err(file_server_error)?
    .map_err(|message| (StatusCode::BAD_REQUEST, message))?;

    let response = RenameBatchOperationResponse {
        successful: statuses.iter().filter(|s| s.is_success()).count(),
        failed: statuses.iter().filter(|s| !s.is_success()).count(),
        statuses,
    };

    if response.failed > 0 {
        Ok((StatusCode::MULTI_STATUS, Json(response)))
    } else {
        Ok((StatusCode::OK, Json(response)))
    }
}

#[utoipa::path(
    delete,
    path = "/",
    tag = "files",
    summary = "Delete files and directories",
    description = "Deletes multiple files and directories in a single atomic transaction.\n\n- Paths ending with `/` are treated as directories and all their contents are deleted recursively.\n- The root directory `\"/\"` cannot be deleted.\n- Per-item failures (e.g., file/directory not found or illegal path) are returned inside the `207 Multi-Status` response body.",
    request_body = DeleteFilesRequest,
    responses(
        (status = 200, description = "All items were deleted successfully", body = BatchOperationResponse),
        (status = 207, description = "Partial or per-item failure (e.g., file or directory does not exist, or attempt to delete root `/`).", body = BatchOperationResponse),
        (status = 400, description = "Bad Request", body = String),
        (status = 409, description = "Concurrent modification conflict after retries (`File metadata changed during this request; reload and retry`)", body = String),
        (status = 500, description = "Internal server / Redis error", body = String)
    )
)]
#[debug_handler]
#[instrument(level = "debug", skip(state))]
pub async fn delete_files(
    State(state): State<AppState>,
    Json(files): Json<DeleteFilesRequest>,
) -> BatchResponse<BatchOperationResponse> {
    debug!("Deleting files {files:?}");
    let ids = files.ids;
    let statuses = retry_file_transaction(&state.file_server, |transaction| {
        let ids = ids.clone();
        Box::pin(async move {
            let mut statuses = Vec::new();
            for id in ids {
                let result = if id.ends_with('/') {
                    transaction.delete_dir(&id).map(|_| ())
                } else {
                    transaction.delete_file(&id).map(|_| ())
                };

                match result {
                    Ok(_) => statuses.push(ItemStatus::Success { id: id.clone() }),
                    Err(err) => statuses.push(ItemStatus::Failure {
                        id: id.clone(),
                        error_type: err.error_type,
                        error: err.message,
                    }),
                }
            }
            Ok(statuses)
        })
    })
    .await
    .map_err(file_server_error)?
    .map_err(|message| (StatusCode::BAD_REQUEST, message))?;

    let response = BatchOperationResponse {
        successful: statuses.iter().filter(|s| s.is_success()).count(),
        failed: statuses.iter().filter(|s| !s.is_success()).count(),
        statuses,
    };

    if response.failed > 0 {
        Ok((StatusCode::MULTI_STATUS, Json(response)))
    } else {
        Ok((StatusCode::OK, Json(response)))
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
            no_file_ext.unwrap_err().message,
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
            result.unwrap_err().message,
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
            fail_result.unwrap_err().message,
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
        assert_eq!(result.unwrap_err().message, "Will not delete root folder");
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
            file.unwrap_err().message,
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
        assert_eq!(err1.error_type, ErrorType::AlreadyExists);

        // 2. Cross directory collision
        let err2 = transaction
            .move_file("/docs/file1.txt", "/archive/file1.txt")
            .unwrap_err();
        assert_eq!(err2.error_type, ErrorType::AlreadyExists);

        // 3. Source does not exist
        let err3 = transaction
            .move_file("/docs/ghost.txt", "/archive/ghost.txt")
            .unwrap_err();
        assert_eq!(err3.error_type, ErrorType::NotFound);
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
        assert_eq!(err1.error_type, ErrorType::MoveIntoSelf);

        // 2. Block moving into its own child (Orphan Tree Bug)
        let err2 = transaction
            .move_dir("/docs/", "/docs/archive/nested/")
            .unwrap_err();
        assert_eq!(err2.error_type, ErrorType::MoveIntoSelf);

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
    async fn test_full_api_multi_status() {
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
        // 1. POST / (Create Directory): 200 OK & 207 MULTI_STATUS
        // ==========================================
        let create_dir_ok_form = multipart::Form::new().text("directory", "/target_dir/");
        let create_dir_ok_res = client
            .post(&format!("{}/", base_url))
            .multipart(create_dir_ok_form)
            .send()
            .await
            .expect("Failed to send create directory request");
        assert_eq!(create_dir_ok_res.status(), StatusCode::OK);
        let create_dir_ok_body: BatchOperationResponse = create_dir_ok_res.json().await.unwrap();
        assert_eq!(create_dir_ok_body.successful, 1);
        assert_eq!(create_dir_ok_body.failed, 0);
        assert!(matches!(
            &create_dir_ok_body.statuses[..],
            [ItemStatus::Success { id }] if id == "/target_dir/"
        ));

        let create_dir_fail_form = multipart::Form::new().text("directory", "/bad@dir/");
        let create_dir_fail_res = client
            .post(&format!("{}/", base_url))
            .multipart(create_dir_fail_form)
            .send()
            .await
            .expect("Failed to send invalid create directory request");
        assert_eq!(create_dir_fail_res.status(), StatusCode::MULTI_STATUS);
        let create_dir_fail_body: BatchOperationResponse =
            create_dir_fail_res.json().await.unwrap();
        assert_eq!(create_dir_fail_body.successful, 0);
        assert_eq!(create_dir_fail_body.failed, 1);
        assert!(matches!(
            &create_dir_fail_body.statuses[..],
            [ItemStatus::Failure {
                id,
                error_type: ErrorType::IllegalPath,
                error
            }] if id == "/bad@dir/" && error.contains("Illegal directory name")
        ));

        // ==========================================
        // 2. POST / (Upload Files): 200 OK & 207 MULTI_STATUS (1+ success, 1+ failure)
        // ==========================================
        let part_a = multipart::Part::bytes(b"File A contents".to_vec())
            .file_name("file_a.txt")
            .mime_str("text/plain")
            .unwrap();
        let part_b = multipart::Part::bytes(b"File B contents".to_vec())
            .file_name("file_b.txt")
            .mime_str("text/plain")
            .unwrap();
        let upload_ok_form = multipart::Form::new()
            .text("directory", "/api_folder")
            .part("files", part_a)
            .part("files", part_b);

        let upload_ok_res = client
            .post(&format!("{}/", base_url))
            .multipart(upload_ok_form)
            .send()
            .await
            .expect("Failed to send initial upload request");
        assert_eq!(upload_ok_res.status(), StatusCode::OK);
        let upload_ok_body: BatchOperationResponse = upload_ok_res.json().await.unwrap();
        assert_eq!(upload_ok_body.successful, 2);
        assert_eq!(upload_ok_body.failed, 0);

        // Upload batch with 1 success (file_c.txt) and 2 failures (duplicate file_a.txt, illegal filename)
        let part_c = multipart::Part::bytes(b"File C contents".to_vec())
            .file_name("file_c.txt")
            .mime_str("text/plain")
            .unwrap();
        let part_dup = multipart::Part::bytes(b"Duplicate A".to_vec())
            .file_name("file_a.txt")
            .mime_str("text/plain")
            .unwrap();
        let part_bad = multipart::Part::bytes(b"No extension".to_vec())
            .file_name("invalid_filename")
            .mime_str("text/plain")
            .unwrap();
        let upload_mixed_form = multipart::Form::new()
            .text("directory", "/api_folder")
            .part("files", part_c)
            .part("files", part_dup)
            .part("files", part_bad);

        let upload_mixed_res = client
            .post(&format!("{}/", base_url))
            .multipart(upload_mixed_form)
            .send()
            .await
            .expect("Failed to send mixed upload request");
        assert_eq!(upload_mixed_res.status(), StatusCode::MULTI_STATUS);
        let upload_mixed_body: BatchOperationResponse = upload_mixed_res.json().await.unwrap();
        assert_eq!(upload_mixed_body.successful, 1);
        assert_eq!(upload_mixed_body.failed, 2);
        assert!(matches!(
            &upload_mixed_body.statuses[0],
            ItemStatus::Success { id } if id == "/api_folder/file_c.txt"
        ));
        assert!(matches!(
            &upload_mixed_body.statuses[1],
            ItemStatus::Failure {
                id,
                error_type: ErrorType::AlreadyExists,
                error
            } if id == "/api_folder/file_a.txt" && error.contains("already exists")
        ));
        assert!(matches!(
            &upload_mixed_body.statuses[2],
            ItemStatus::Failure {
                id,
                error_type: ErrorType::IllegalPath,
                error
            } if id == "/api_folder/invalid_filename" && error.contains("Illegal file name")
        ));

        // ==========================================
        // 3. PUT / (Rename / Move): 200 OK & 207 MULTI_STATUS (1+ success, 1+ failure)
        // ==========================================
        let rename_ok_payload = RenameRequest {
            ids_from: vec!["/api_folder/file_a.txt".to_string()],
            ids_to: vec!["/api_folder/file_a_renamed.txt".to_string()],
        };
        let rename_ok_res = client
            .put(&format!("{}/", base_url))
            .json(&rename_ok_payload)
            .send()
            .await
            .expect("Failed to send rename OK request");
        assert_eq!(rename_ok_res.status(), StatusCode::OK);
        let rename_ok_body: RenameBatchOperationResponse = rename_ok_res.json().await.unwrap();
        assert_eq!(rename_ok_body.successful, 1);
        assert_eq!(rename_ok_body.failed, 0);
        assert!(matches!(
            &rename_ok_body.statuses[..],
            [RenameItemStatus::Success { from, to }]
                if from == "/api_folder/file_a.txt" && to == "/api_folder/file_a_renamed.txt"
        ));

        // Mixed rename batch:
        // - Success: move /api_folder/file_b.txt -> /target_dir/file_b_moved.txt
        // - Success: rename directory /target_dir/ -> /moved_dir/
        // - Failure (NotFound): non-existent source file
        // - Failure (AlreadyExists): destination file already exists (/api_folder/file_a_renamed.txt)
        // - Failure (MixedFileAndDirectory): mixing file and directory paths at same index
        // - Failure (MoveIntoSelf): moving directory into itself/subdirectory
        // - Failure (CannotModifyRoot): moving root directory "/"
        let rename_mixed_payload = RenameRequest {
            ids_from: vec![
                "/api_folder/file_b.txt".to_string(),
                "/target_dir/".to_string(),
                "/api_folder/non_existent.txt".to_string(),
                "/api_folder/file_c.txt".to_string(),
                "/api_folder/file_c.txt".to_string(),
                "/moved_dir/".to_string(),
                "/".to_string(),
            ],
            ids_to: vec![
                "/target_dir/file_b_moved.txt".to_string(),
                "/moved_dir/".to_string(),
                "/api_folder/still_missing.txt".to_string(),
                "/api_folder/file_a_renamed.txt".to_string(),
                "/moved_dir/".to_string(),
                "/moved_dir/sub/".to_string(),
                "/root_copy/".to_string(),
            ],
        };
        let rename_mixed_res = client
            .put(&format!("{}/", base_url))
            .json(&rename_mixed_payload)
            .send()
            .await
            .expect("Failed to send mixed rename request");
        assert_eq!(rename_mixed_res.status(), StatusCode::MULTI_STATUS);
        let rename_mixed_body: RenameBatchOperationResponse =
            rename_mixed_res.json().await.unwrap();
        assert_eq!(rename_mixed_body.successful, 2);
        assert_eq!(rename_mixed_body.failed, 5);
        assert!(matches!(
            &rename_mixed_body.statuses[0],
            RenameItemStatus::Success { from, to }
                if from == "/api_folder/file_b.txt" && to == "/target_dir/file_b_moved.txt"
        ));
        assert!(matches!(
            &rename_mixed_body.statuses[1],
            RenameItemStatus::Success { from, to }
                if from == "/target_dir/" && to == "/moved_dir/"
        ));
        assert!(matches!(
            &rename_mixed_body.statuses[2],
            RenameItemStatus::Failure {
                from,
                to,
                error_type: ErrorType::NotFound,
                error
            } if from == "/api_folder/non_existent.txt"
                && to == "/api_folder/still_missing.txt"
                && error.contains("does not exist")
        ));
        assert!(matches!(
            &rename_mixed_body.statuses[3],
            RenameItemStatus::Failure {
                from,
                to,
                error_type: ErrorType::AlreadyExists,
                error
            } if from == "/api_folder/file_c.txt"
                && to == "/api_folder/file_a_renamed.txt"
                && error.contains("already exists")
        ));
        assert!(matches!(
            &rename_mixed_body.statuses[4],
            RenameItemStatus::Failure {
                from,
                to,
                error_type: ErrorType::MixedFileAndDirectory,
                error
            } if from == "/api_folder/file_c.txt"
                && to == "/moved_dir/"
                && error.contains("Cannot mix file and directories")
        ));
        assert!(matches!(
            &rename_mixed_body.statuses[5],
            RenameItemStatus::Failure {
                from,
                to,
                error_type: ErrorType::MoveIntoSelf,
                error
            } if from == "/moved_dir/"
                && to == "/moved_dir/sub/"
                && error.contains("Cannot move a directory into itself")
        ));
        assert!(matches!(
            &rename_mixed_body.statuses[6],
            RenameItemStatus::Failure {
                from,
                to,
                error_type: ErrorType::CannotModifyRoot,
                error
            } if from == "/"
                && to == "/root_copy/"
                && error.contains("Cannot move root folder")
        ));

        // ==========================================
        // 4. GET /tree & GET /list: Verify state after partial batches
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
        assert_eq!(api_folder.files.len(), 2);
        assert_eq!(api_folder.files[0].name, "file_a_renamed.txt");
        assert_eq!(api_folder.files[1].name, "file_c.txt");

        let moved_dir = tree_json
            .directories
            .iter()
            .find(|d| d.name == "moved_dir")
            .unwrap();
        assert_eq!(moved_dir.files.len(), 1);
        assert_eq!(moved_dir.files[0].name, "file_b_moved.txt");

        let list_res = client
            .get(&format!("{}/list", base_url))
            .send()
            .await
            .expect("Failed to fetch list");
        assert_eq!(list_res.status(), StatusCode::OK);
        let list_json: ListView = list_res.json().await.unwrap();
        assert_eq!(list_json.0.len(), 5); // 2 folders (/api_folder, /moved_dir) + 3 files

        // ==========================================
        // 5. DELETE / (Delete Files & Directories): 207 MULTI_STATUS & 200 OK
        // ==========================================
        // Mixed delete batch:
        // - Success: delete /api_folder/file_a_renamed.txt
        // - Failure (NotFound): delete non-existent file /api_folder/ghost.txt
        // - Failure (CannotModifyRoot): attempt to delete root directory "/"
        // - Failure (NotFound): delete non-existent directory /ghost_dir/
        // - Failure (IllegalPath): delete invalid file path
        let delete_mixed_payload = DeleteFilesRequest {
            ids: vec![
                "/api_folder/file_a_renamed.txt".to_string(),
                "/api_folder/ghost.txt".to_string(),
                "/".to_string(),
                "/ghost_dir/".to_string(),
                "/bad@file.txt".to_string(),
            ],
        };
        let delete_mixed_res = client
            .delete(&format!("{}/", base_url))
            .json(&delete_mixed_payload)
            .send()
            .await
            .expect("Failed to send mixed delete request");
        assert_eq!(delete_mixed_res.status(), StatusCode::MULTI_STATUS);
        let delete_mixed_body: BatchOperationResponse = delete_mixed_res.json().await.unwrap();
        assert_eq!(delete_mixed_body.successful, 1);
        assert_eq!(delete_mixed_body.failed, 4);
        assert!(matches!(
            &delete_mixed_body.statuses[0],
            ItemStatus::Success { id } if id == "/api_folder/file_a_renamed.txt"
        ));
        assert!(matches!(
            &delete_mixed_body.statuses[1],
            ItemStatus::Failure {
                id,
                error_type: ErrorType::NotFound,
                error
            } if id == "/api_folder/ghost.txt" && error.contains("does not exists")
        ));
        assert!(matches!(
            &delete_mixed_body.statuses[2],
            ItemStatus::Failure {
                id,
                error_type: ErrorType::CannotModifyRoot,
                error
            } if id == "/" && error.contains("Will not delete root folder")
        ));
        assert!(matches!(
            &delete_mixed_body.statuses[3],
            ItemStatus::Failure {
                id,
                error_type: ErrorType::NotFound,
                error
            } if id == "/ghost_dir/" && error.contains("Directory does not exist")
        ));
        assert!(matches!(
            &delete_mixed_body.statuses[4],
            ItemStatus::Failure {
                id,
                error_type: ErrorType::IllegalPath,
                error
            } if id == "/bad@file.txt" && error.contains("Illegal file name")
        ));

        // Clean delete batch (200 OK): delete remaining file and directories
        let delete_ok_payload = DeleteFilesRequest {
            ids: vec![
                "/api_folder/file_c.txt".to_string(),
                "/moved_dir/".to_string(),
                "/api_folder/".to_string(),
            ],
        };
        let delete_ok_res = client
            .delete(&format!("{}/", base_url))
            .json(&delete_ok_payload)
            .send()
            .await
            .expect("Failed to send OK delete request");
        assert_eq!(delete_ok_res.status(), StatusCode::OK);
        let delete_ok_body: BatchOperationResponse = delete_ok_res.json().await.unwrap();
        assert_eq!(delete_ok_body.successful, 3);
        assert_eq!(delete_ok_body.failed, 0);

        let final_tree_res = client
            .get(&format!("{}/tree", base_url))
            .send()
            .await
            .unwrap();
        let final_tree_json: TreeDirectory = final_tree_res.json().await.unwrap();
        assert!(final_tree_json.directories.is_empty());
        assert!(final_tree_json.files.is_empty());

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
                    transaction
                        .add_file("/retry-left.txt", b"left")
                        .await
                        .map_err(|f| f.message)?;
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
                    transaction
                        .add_file("/retry-right.txt", b"right")
                        .await
                        .map_err(|f| f.message)?;
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
                    let file = transaction
                        .add_file(path, b"left")
                        .await
                        .map_err(|f| f.message)?;
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
                    let file = transaction
                        .add_file(path, b"right")
                        .await
                        .map_err(|f| f.message)?;
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
        if let Err(err) = post_abort {
            assert_eq!(err.error_type, ErrorType::InternalError);
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
