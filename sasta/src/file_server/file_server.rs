use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::LazyLock,
};

use chrono::{DateTime, Local};
use redis::{Client, Value, aio::MultiplexedConnection};
use regex::Regex;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use tokio::{fs, io::AsyncWriteExt};
use tracing::{error, info, warn};
use uuid::Uuid;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct File {
    pub name: String,
    /// Actual filename on disk (`{UUID}.{ext}`).
    pub file_server: String,
    /// File path through the built file tree.
    pub path: String,
    pub size: usize,
    pub date: DateTime<Local>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Directory {
    pub name: String,
    pub path: String,
    pub files: HashMap<String, File>,
    pub children: HashMap<String, Directory>,
}

#[derive(Clone)]
pub struct FileServer {
    pub path: PathBuf,
    client: Client,
    key: String,
}

pub struct FileTransaction {
    path: PathBuf,
    key: String,
    con: MultiplexedConnection,
    root: Directory,
    created_files: Vec<PathBuf>,
    deleted_files: Vec<PathBuf>,
    finished: bool,
}

#[derive(Debug)]
pub enum FileServerError {
    Conflict,
    TransactionClosed,
    Redis(String),
    InvalidData(String),
}

impl std::fmt::Display for FileServerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Conflict => write!(
                f,
                "File metadata changed during this request; reload and retry"
            ),
            Self::TransactionClosed => write!(f, "File transaction is already closed"),
            Self::Redis(message) | Self::InvalidData(message) => f.write_str(message),
        }
    }
}

pub static FILE_PATH_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^/[\w/_\- ]*(\w+\.\w+)$").unwrap());

pub static DIR_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^/[(\w/_\- )]+/$|^/$").unwrap());

#[derive(Debug, Clone, PartialEq)]
pub enum VirtualPath<'a> {
    Root,
    File(Vec<&'a str>),
    Directory(Vec<&'a str>),
}

impl<'a> VirtualPath<'a> {
    pub fn parse_file(path: &'a str) -> Result<Self, String> {
        Self::internal_parse(path, true)
    }

    pub fn parse_dir(path: &'a str) -> Result<Self, String> {
        Self::internal_parse(path, false)
    }

    fn internal_parse(path: &'a str, is_file: bool) -> Result<Self, String> {
        let path = path.trim();

        if path == "/" {
            if is_file {
                return Err("Root path '/' cannot be a file".to_string());
            }
            return Ok(VirtualPath::Root);
        }

        if is_file && !FILE_PATH_REGEX.is_match(path) {
            return Err(format!("Illegal file name '{path}'"));
        } else if !is_file && !DIR_REGEX.is_match(path) {
            return Err(format!("Illegal directory name '{path}'"));
        }

        let components: Vec<&'a str> = path.split('/').filter(|s| !s.is_empty()).collect();
        if is_file {
            Ok(VirtualPath::File(components))
        } else {
            Ok(VirtualPath::Directory(components))
        }
    }

    /// Returns the full path as an array slice.
    pub fn path(&self) -> &[&'a str] {
        match self {
            VirtualPath::Root => &[],
            VirtualPath::File(components) | VirtualPath::Directory(components) => components,
        }
    }

    /// Returns just the parent segments
    pub fn parent(&self) -> &[&'a str] {
        match self {
            VirtualPath::Root => &[],
            VirtualPath::File(components) | VirtualPath::Directory(components) => {
                if components.is_empty() {
                    &[]
                } else {
                    &components[..components.len() - 1]
                }
            }
        }
    }

    pub fn name(&self) -> &'a str {
        match self {
            VirtualPath::Root => "root",
            VirtualPath::File(components) | VirtualPath::Directory(components) => {
                components.last().unwrap_or(&"root")
            }
        }
    }

    /// Reconstructs a clean path.
    pub fn to_string_path(&self) -> String {
        match self {
            VirtualPath::Root => "/".to_string(),
            VirtualPath::File(components) | VirtualPath::Directory(components) => {
                format!("/{}", components.join("/"))
            }
        }
    }
}

impl FileServer {
    pub async fn new(redis_url: &str, path: impl Into<PathBuf>) -> Self {
        Self::new_with_key(redis_url, path, "files").await
    }

    pub async fn new_with_key(
        redis_url: &str,
        path: impl Into<PathBuf>,
        key: impl Into<String>,
    ) -> Self {
        let path = path.into();
        let key = key.into();
        fs::create_dir_all(&path)
            .await
            .expect("create file storage directory");
        let client = Client::open(redis_url).expect("invalid REDIS_URL");
        let mut con = client
            .get_multiplexed_async_connection()
            .await
            .expect("connect to Redis for file server initialization");
        let _: Option<String> = redis::cmd("JSON.SET")
            .arg(&key)
            .arg(".")
            .arg(r#"{"name":"","path":"/","files":{},"children":{}}"#)
            .arg("NX")
            .query_async(&mut con)
            .await
            .expect("initialize file metadata document");
        Self { client, path, key }
    }

    pub fn metadata_key(&self) -> &str {
        &self.key
    }

    pub async fn read_tree(&self) -> Result<Directory, FileServerError> {
        let mut con = self.connection().await?;
        read_json_path(&mut con, &self.key, "$")
            .await?
            .ok_or_else(|| {
                FileServerError::InvalidData("Redis file metadata document is missing".into())
            })
    }

    pub async fn get_file(&self, file_path: &str) -> Result<Option<String>, FileServerError> {
        let Ok(path) = VirtualPath::parse_file(file_path) else {
            return Ok(None);
        };
        let mut con = self.connection().await?;
        let file: Option<File> =
            read_json_path(&mut con, &self.key, &Self::file_json_path(path)).await?;
        Ok(file.map(|file| file.file_server))
    }

    fn file_json_path(path: VirtualPath) -> String {
        let mut path_redis = Self::json_directory_path(path.parent());
        path_redis.push_str("[\"files\"][");
        path_redis.push_str(&serde_json::to_string(path.name()).expect("serialize path component"));
        path_redis.push(']');
        path_redis
    }

    fn json_directory_path(components: &[&str]) -> String {
        let mut path = "$".to_string();
        for component in components {
            path.push_str("[\"children\"][");
            path.push_str(&serde_json::to_string(component).expect("serialize path component"));
            path.push(']');
        }
        path
    }

    pub async fn begin_transaction(&self) -> Result<FileTransaction, FileServerError> {
        let con = self.connection().await?;
        FileTransaction::new(self.path.clone(), self.key.clone(), con).await
    }

    async fn connection(&self) -> Result<MultiplexedConnection, FileServerError> {
        self.client
            .get_multiplexed_async_connection()
            .await
            .map_err(|error| FileServerError::Redis(error.to_string()))
    }
}

async fn read_json_path<T: DeserializeOwned>(
    con: &mut MultiplexedConnection,
    key: &str,
    path: &str,
) -> Result<Option<T>, FileServerError> {
    let value: Option<String> = redis::cmd("JSON.GET")
        .arg(key)
        .arg(path)
        .query_async(con)
        .await
        .map_err(|error| FileServerError::Redis(error.to_string()))?;
    let Some(value) = value else {
        return Ok(None);
    };

    let parsed: serde_json::Value = serde_json::from_str(&value)
        .map_err(|error| FileServerError::InvalidData(error.to_string()))?;
    let selected = match parsed {
        serde_json::Value::Array(mut values) => {
            if values.is_empty() {
                return Ok(None);
            }
            values.remove(0)
        }
        value => value,
    };
    serde_json::from_value(selected)
        .map(Some)
        .map_err(|error| FileServerError::InvalidData(error.to_string()))
}

fn child_virtual_path(parent: &str, name: &str) -> String {
    if parent == "/" {
        format!("/{name}")
    } else {
        format!("{parent}/{name}")
    }
}

impl FileTransaction {
    async fn new(
        path: PathBuf,
        key: String,
        mut con: MultiplexedConnection,
    ) -> Result<Self, FileServerError> {
        redis::cmd("WATCH")
            .arg(&key)
            .query_async::<()>(&mut con)
            .await
            .map_err(|error| FileServerError::Redis(error.to_string()))?;

        let root = match read_json_path(&mut con, &key, "$").await {
            Ok(Some(root)) => root,
            Ok(None) => {
                let _ = redis::cmd("UNWATCH").query_async::<()>(&mut con).await;
                return Err(FileServerError::InvalidData(
                    "Redis file metadata document is missing".into(),
                ));
            }
            Err(error) => {
                let _ = redis::cmd("UNWATCH").query_async::<()>(&mut con).await;
                return Err(error);
            }
        };

        Ok(FileTransaction {
            path,
            key,
            con,
            root,
            created_files: Vec::new(),
            deleted_files: Vec::new(),
            finished: false,
        })
    }

    fn ensure_open(&self) -> Result<(), String> {
        if self.finished {
            Err(FileServerError::TransactionClosed.to_string())
        } else {
            Ok(())
        }
    }

    /// Add a file and create any missing parent directories. Metadata stays staged until `write`.
    pub async fn add_file(&mut self, file_path: &str, content: &[u8]) -> Result<File, String> {
        self.ensure_open()?;
        let path = VirtualPath::parse_file(file_path)?;
        info!("Adding file {}", path.to_string_path());
        self.create_up_to_dir(path.parent())?;

        let normalized_path = path.to_string_path();
        let parent = path.parent();
        if self
            .find_directory(parent)
            .is_some_and(|directory| directory.files.contains_key(path.name()))
        {
            return Err(format!("File {} already exists", normalized_path));
        }

        let extension = Path::new(path.name())
            .extension()
            .and_then(|extension| extension.to_str())
            .unwrap_or("txt")
            .to_string();
        let file = File {
            name: path.name().to_string(),
            file_server: format!("{}.{}", Uuid::new_v4(), extension),
            path: normalized_path.clone(),
            size: content.len(),
            date: Local::now(),
        };

        let disk_path = self.path.join(&file.file_server);
        match fs::File::create(&disk_path).await {
            Ok(mut disk_file) => {
                if let Err(error) = disk_file.write_all(content).await {
                    let _ = fs::remove_file(&disk_path).await;
                    return Err(error.to_string());
                }
            }
            Err(error) => return Err(error.to_string()),
        }
        self.created_files.push(disk_path);

        let directory = self
            .find_directory_mut(parent)
            .expect("parent directory was created");
        directory.files.insert(file.name.clone(), file.clone());
        Ok(file)
    }

    pub fn move_file(&mut self, old_path: &str, new_path: &str) -> Result<File, String> {
        self.ensure_open()?;
        info!("Moving file {} to {}", old_path, new_path);
        let old = VirtualPath::parse_file(old_path)?;
        let new = VirtualPath::parse_file(new_path)?;
        if self.find_directory(old.parent()).is_none()
            || self.find_directory(new.parent()).is_none()
        {
            return Err("Directory does not exist".into());
        }
        let mut file = self
            .find_directory(old.parent())
            .and_then(|directory| directory.files.get(old.name()).cloned())
            .ok_or_else(|| format!("File {} does not exist", old.to_string_path()))?;
        if self
            .find_directory(new.parent())
            .and_then(|directory| directory.files.get(new.name()))
            .is_some()
        {
            return Err(format!(
                "Cannot move file since {} already exists",
                new.to_string_path()
            ));
        }

        file.name = new.name().to_string();
        file.path = new.to_string_path();
        self.find_directory_mut(old.parent())
            .expect("source parent directory was checked")
            .files
            .remove(old.name());
        self.find_directory_mut(new.parent())
            .expect("destination parent directory was checked")
            .files
            .insert(file.name.clone(), file.clone());
        Ok(file)
    }

    pub fn delete_file(&mut self, file_path: &str) -> Result<File, String> {
        self.ensure_open()?;
        let path = VirtualPath::parse_file(file_path)?;
        let file = self
            .find_directory_mut(path.parent())
            .and_then(|directory| directory.files.remove(path.name()))
            .ok_or_else(|| format!("File {} does not exists", path.to_string_path()))?;

        self.deleted_files.push(self.path.join(&file.file_server));
        Ok(file)
    }

    pub fn add_dir(&mut self, dir_path: &str) -> Result<(), String> {
        self.ensure_open()?;
        info!("Adding directory {}", dir_path);
        let path = VirtualPath::parse_dir(dir_path)?;
        self.create_up_to_dir(path.path())
    }

    pub fn move_dir(&mut self, old_path: &str, new_path: &str) -> Result<Directory, String> {
        self.ensure_open()?;
        info!("Moving directory {} to {}", old_path, new_path);
        let old = VirtualPath::parse_dir(old_path)?;
        let new = VirtualPath::parse_dir(new_path)?;

        if old.path().is_empty() {
            return Err("Cannot move root folder".into());
        }
        let old_normalized = old.to_string_path();
        let new_normalized = new.to_string_path();
        if old_normalized == new_normalized
            || new_normalized.starts_with(&format!("{old_normalized}/"))
        {
            error!("Cannot move directory into itself");
            return Err("Cannot move a directory into itself or its subdirectories.".into());
        }
        if self.find_directory(old.parent()).is_none()
            || self.find_directory(new.parent()).is_none()
        {
            return Err("Directory does not exist".into());
        }
        let mut directory = self
            .find_directory(&old.path())
            .cloned()
            .ok_or_else(|| format!("Directory {} does not exist", old_normalized))?;
        if self.find_directory(&new.path()).is_some() {
            return Err(format!(
                "Cannot move directory since {} already exists",
                new_normalized
            ));
        }

        directory.name = new.name().to_string();
        Self::update_directory_path(&mut directory, &new_normalized);
        self.find_directory_mut(old.parent())
            .expect("source parent directory was checked")
            .children
            .remove(old.name());
        self.find_directory_mut(new.parent())
            .expect("destination parent directory was checked")
            .children
            .insert(directory.name.clone(), directory.clone());
        Ok(directory)
    }

    fn update_directory_path(directory: &mut Directory, path: &str) {
        directory.path = path.to_string();
        let child_names = directory.children.keys().cloned().collect::<Vec<_>>();
        for name in child_names {
            if let Some(child) = directory.children.get_mut(&name) {
                Self::update_directory_path(child, &child_virtual_path(path, &name));
            }
        }
        for file in directory.files.values_mut() {
            file.path = child_virtual_path(path, &file.name);
        }
    }

    pub fn delete_dir(&mut self, dir_path: &str) -> Result<Directory, String> {
        self.ensure_open()?;
        info!("Deleting directory {}", dir_path);
        let path = VirtualPath::parse_dir(dir_path)?;
        if path.path().is_empty() {
            return Err("Will not delete root folder".into());
        }
        let parent = path.parent();
        let directory = self
            .find_directory_mut(parent)
            .and_then(|parent| parent.children.remove(path.name()))
            .ok_or_else(|| "Directory does not exist".to_string())?;

        let mut files = Vec::new();
        Self::collect_files(&directory, &mut files);
        self.deleted_files.extend(
            files
                .into_iter()
                .map(|file| self.path.join(file.file_server)),
        );
        Ok(directory)
    }

    fn find_directory<'a>(&'a self, components: &[&str]) -> Option<&'a Directory> {
        let mut directory = &self.root;
        for component in components {
            directory = directory.children.get(*component)?;
        }
        Some(directory)
    }

    fn find_directory_mut<'a>(&'a mut self, components: &[&str]) -> Option<&'a mut Directory> {
        let mut directory = &mut self.root;
        for component in components {
            directory = directory.children.get_mut(*component)?;
        }
        Some(directory)
    }

    fn collect_files(directory: &Directory, files: &mut Vec<File>) {
        files.extend(directory.files.values().cloned());
        for child in directory.children.values() {
            Self::collect_files(child, files);
        }
    }

    fn create_up_to_dir(&mut self, components: &[&str]) -> Result<(), String> {
        let mut current = Vec::<&str>::new();
        for component in components {
            current.push(component);
            if self.find_directory(&current).is_none() {
                let directory = Directory {
                    name: component.to_string(),
                    path: format!("/{}", current.join("/")),
                    files: HashMap::new(),
                    children: HashMap::new(),
                };
                let parent = &current[..current.len() - 1];
                self.find_directory_mut(parent)
                    .expect("parent directory does not exist")
                    .children
                    .insert((*component).to_string(), directory);
            }
        }
        Ok(())
    }

    pub async fn abort_transaction(&mut self) {
        if self.finished {
            return;
        }
        self.finished = true;
        let _ = redis::cmd("UNWATCH").query_async::<()>(&mut self.con).await;
        self.cleanup_created_files().await;
        self.deleted_files.clear();
    }

    pub async fn write(&mut self) -> Result<(), FileServerError> {
        if self.finished {
            return Err(FileServerError::TransactionClosed);
        }
        self.finished = true;

        let tree = match serde_json::to_string(&self.root) {
            Ok(tree) => tree,
            Err(error) => {
                self.cleanup_created_files().await;
                self.deleted_files.clear();
                return Err(FileServerError::InvalidData(error.to_string()));
            }
        };

        if let Err(error) = redis::cmd("MULTI")
            .query_async::<Value>(&mut self.con)
            .await
        {
            self.cleanup_created_files().await;
            self.deleted_files.clear();
            return Err(FileServerError::Redis(error.to_string()));
        }

        if let Err(error) = redis::cmd("JSON.SET")
            .arg(&self.key)
            .arg(".")
            .arg(tree)
            .query_async::<Value>(&mut self.con)
            .await
        {
            let _ = redis::cmd("DISCARD")
                .query_async::<Value>(&mut self.con)
                .await;
            self.cleanup_created_files().await;
            self.deleted_files.clear();
            return Err(FileServerError::Redis(error.to_string()));
        }

        let result: Option<Vec<Value>> = match redis::cmd("EXEC").query_async(&mut self.con).await {
            Ok(result) => result,
            Err(error) => {
                // Redis may have committed even if its response was lost.
                self.created_files.clear();
                self.deleted_files.clear();
                return Err(FileServerError::Redis(error.to_string()));
            }
        };

        match result {
            Some(results) if results.len() == 1 => match &results[0] {
                Value::ServerError(error) => {
                    self.cleanup_created_files().await;
                    self.deleted_files.clear();
                    Err(FileServerError::Redis(format!(
                        "Redis JSON.SET failed: {error}"
                    )))
                }
                Value::Nil => {
                    self.cleanup_created_files().await;
                    self.deleted_files.clear();
                    Err(FileServerError::Redis(
                        "Redis JSON.SET did not update the file tree".into(),
                    ))
                }
                _ => {
                    self.created_files.clear();
                    for file in self.deleted_files.drain(..) {
                        if let Err(error) = fs::remove_file(file).await {
                            if error.kind() != std::io::ErrorKind::NotFound {
                                warn!("Could not remove deleted file from shared storage: {error}");
                            }
                        }
                    }
                    Ok(())
                }
            },
            Some(results) => {
                // An unexpected EXEC response is ambiguous; keep both sets of bytes safe.
                self.created_files.clear();
                self.deleted_files.clear();
                Err(FileServerError::Redis(format!(
                    "Redis EXEC returned {} results for one JSON.SET",
                    results.len()
                )))
            }
            None => {
                self.cleanup_created_files().await;
                self.deleted_files.clear();
                Err(FileServerError::Conflict)
            }
        }
    }

    async fn cleanup_created_files(&mut self) {
        for file in self.created_files.drain(..) {
            if let Err(error) = fs::remove_file(file).await {
                if error.kind() != std::io::ErrorKind::NotFound {
                    warn!("Could not clean up upload after failed metadata commit: {error}");
                }
            }
        }
    }
}

impl Drop for FileTransaction {
    fn drop(&mut self) {
        if !self.finished {
            for file in self.created_files.drain(..) {
                if let Err(error) = std::fs::remove_file(file) {
                    if error.kind() != std::io::ErrorKind::NotFound {
                        warn!(
                            "Could not clean up staged upload when dropping transaction: {error}"
                        );
                    }
                }
            }
        }
    }
}
