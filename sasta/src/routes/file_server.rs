use std::collections::LinkedList;

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
    file_server::file_server::{Directory, File},
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

#[derive(Serialize, Debug, ToSchema)]
pub struct ListView(Vec<ListViewItem>);

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
    let mut file_server = state.file_server.lock().await;

    let upload = match FileUpload::from_multipart(multipart).await {
        Ok(u) => u,
        Err(message) => {
            return Err((StatusCode::BAD_REQUEST, message));
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
                    format!("{message} ({})", upload.directory),
                ));
            }
        }
    }

    let mut errors = Vec::new();

    for file_item in upload.files {
        if let Err(message) = file_server
            .add_file(
                format!("{}/{}", upload.directory, file_item.name),
                file_item.content,
            )
            .await
        {
            errors.push(message);
        }
    }

    file_server.write().await;

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
    let files = (&state.file_server.lock().await.root).into();
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
    let files = (&state.file_server.lock().await.root).into();
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
            let f = ServeFile::new(file_server.path.join(&p));
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
    let mut file_server = state.file_server.lock().await;

    let mut errors = Vec::new();

    for (from, to) in files.ids_from.iter().zip(files.ids_to.iter()) {
        if from.ends_with('/') && to.ends_with('/') {
            match file_server.move_dir(from, to).await {
                Ok(_) => (),
                Err(message) => errors.push(message),
            }
        } else if !from.ends_with('/') && !to.ends_with('/') {
            match file_server.move_file(from, to).await {
                Ok(_) => (),
                Err(message) => errors.push(message),
            }
        } else {
            errors.push(
                "Cannot mix file and directories on the corresponding indexes of the arrays fields"
                    .to_string(),
            )
        }
    }

    file_server.write().await;

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
    let mut file_server = state.file_server.lock().await;

    let mut errors = Vec::new();

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
    use crate::file_server::file_server::{DIR_REGEX, FILE_PATH_REGEX, FileServer};
    use crate::store::store::Store;

    use super::*;
    use axum::serve;
    use reqwest::multipart;
    use std::path::Path;
    use std::sync::Arc;
    use tokio::fs;
    use tokio::net::TcpListener;
    use tokio::sync::Mutex as AsyncMutex;

    const FILE_PATH: &'static str = "./test_files";

    /// Helper function
    async fn setup_test_server() -> FileServer {
        FileServer::new("not used since in test environment", FILE_PATH).await
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
        let server = setup_test_server().await;
        let tree: TreeDirectory = (&server.root).into();
        assert_eq!(tree.id, "/");
    }

    #[tokio::test]
    async fn test_add_file_creates_directories() {
        let mut server = setup_test_server().await;

        let path = "/test_folder/nested/file.txt".to_string();
        let file = server
            .add_file(path.clone(), vec![0x00; 1024])
            .await
            .expect("Failed to add file");

        assert_eq!(file.name, "file.txt");
        assert_eq!(file.path, "/test_folder/nested/file.txt");
        assert_eq!(file.size, 1024);

        let no_file_ext = server.add_file("/test".to_string(), vec![]).await;
        assert_eq!(no_file_ext.unwrap_err(), "Illegal file name".to_string());

        let tree: TreeDirectory = (&server.root).into();

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
    }

    #[tokio::test]
    async fn test_add_duplicate_file_fails() {
        let mut server = setup_test_server().await;

        let path = "/duplicates/file.txt".to_string();

        let _ = server.add_file(path.clone(), vec![]).await.unwrap();

        let result = server.add_file(path.clone(), vec![]).await;
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

        let file = server.add_file(path.clone(), vec![]).await.unwrap();

        let disk_path = format!("{FILE_PATH}/{}", file.file_server);
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

        let dir_path = "/my_folder/child_folder/".to_string();
        server.add_dir(&dir_path).await.expect("Failed to add dir");

        let file = server
            .add_file("/my_folder/child_folder/test.txt".to_string(), vec![])
            .await
            .unwrap();
        let disk_path = format!("{FILE_PATH}/{}", file.file_server);
        fs::write(&disk_path, b"data").await.unwrap();

        let deleted_dir = server
            .delete_dir("/my_folder/".to_string())
            .await
            .expect("Failed to delete dir");
        assert_eq!(deleted_dir.name, "my_folder");

        assert!(!Path::new(&disk_path).exists());

        let tree: TreeDirectory = (&server.root).into();
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

        let file = server.add_file(weird_path, vec![]).await.unwrap();
        assert_eq!(file.path, "/weird/path/file.txt");

        let tree: TreeDirectory = (&server.root).into();
        let weird_dir = tree.directories.iter().find(|d| d.name == "weird").unwrap();
        let path_dir = weird_dir
            .directories
            .iter()
            .find(|d| d.name == "path")
            .unwrap();

        assert_eq!(path_dir.files[0].name, "file.txt");

        let weird_path = "/../wonky/.file/....path..txt".to_string();

        let file = server.add_file(weird_path, vec![]).await;
        assert_eq!(file.unwrap_err(), "Illegal file name");
    }

    #[tokio::test]
    async fn test_move_file_cross_directory_and_rename() {
        let mut server = setup_test_server().await;
        server.add_dir(&"/folder_a/".to_string()).await.unwrap();
        server.add_dir(&"/folder_b/".to_string()).await.unwrap();
        server
            .add_file("/folder_a/test.txt".to_string(), vec![])
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

        let tree: TreeDirectory = (&server.root).into();
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
        server.add_dir(&"/docs/".to_string()).await.unwrap();
        server
            .add_file("/docs/apple.txt".to_string(), vec![])
            .await
            .unwrap();
        server
            .add_file("/docs/banana.txt".to_string(), vec![])
            .await
            .unwrap();
        server
            .add_file("/docs/zebra.txt".to_string(), vec![])
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

        let tree: TreeDirectory = (&server.root).into();
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
            .add_file("/docs/file1.txt".to_string(), vec![])
            .await
            .unwrap();
        server
            .add_file("/docs/file2.txt".to_string(), vec![])
            .await
            .unwrap();
        server
            .add_file("/archive/file1.txt".to_string(), vec![])
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
            .add_file("/parent/child/deep/file.txt".to_string(), vec![])
            .await
            .unwrap();
        server.add_dir(&"/archive/".to_string()).await.unwrap();

        let moved_dir = server
            .move_dir(
                &"/parent/child/".to_string(),
                &"/archive/renamed_child/".to_string(),
            )
            .await
            .unwrap();

        assert_eq!(moved_dir.path, "/archive/renamed_child");

        let tree: TreeDirectory = (&server.root).into();
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
        server.add_dir(&"/docs/archive/".to_string()).await.unwrap();

        // 1. Block moving into itself
        let err1 = server
            .move_dir(&"/docs/".to_string(), &"/docs/".to_string())
            .await
            .unwrap_err();
        assert!(err1.contains("Cannot move a directory into itself"));

        // 2. Block moving into its own child (Orphan Tree Bug)
        let err2 = server
            .move_dir(&"/docs/".to_string(), &"/docs/archive/nested/".to_string())
            .await
            .unwrap_err();
        assert!(err2.contains("Cannot move a directory into itself"));

        // 3. DO NOT block moving into a different folder with a similar prefix name!
        // This ensures new_dir_path.starts_with(&format!("{}/", old_dir_path)) is working perfectly.
        server.add_dir(&"/docs_new/".to_string()).await.unwrap();
        let success = server
            .move_dir(&"/docs/".to_string(), &"/docs_new/docs/".to_string())
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
            store: Arc::new(tokio::sync::Mutex::new(
                Store::new("again, not used in test environment").await,
            )),
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
