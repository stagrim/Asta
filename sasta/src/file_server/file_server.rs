use std::{
    collections::HashMap,
    io::Write,
    path::{Path, PathBuf},
    sync::LazyLock,
};

use chrono::{DateTime, Local};
use redis::{Client, Value, aio::MultiplexedConnection};
use regex::Regex;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use tokio::fs;
use tracing::warn;
use tracing::{error_span, info_span};
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

enum Mutation {
    Set { path: String, value: String },
    Delete { path: String },
}

pub struct FileTransaction {
    path: PathBuf,
    key: String,
    con: MultiplexedConnection,
    mutations: Vec<Mutation>,
    directory_overrides: HashMap<String, Option<Directory>>,
    file_overrides: HashMap<String, Option<File>>,
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
    // TODO: Use this internally, but use external, parse_file/parse_dir instead
    pub fn parse(path: &'a str, is_file: bool) -> Result<Self, String> {
        let path = path.trim();

        if path == "/" {
            if is_file {
                return Err("Root path '/' cannot be a file".to_string());
            }
            return Ok(VirtualPath::Root);
        }

        if is_file && !FILE_PATH_REGEX.is_match(path) {
            return Err("Illegal file name".to_string());
        } else if !is_file && !DIR_REGEX.is_match(path) {
            return Err("Illegal directory name".to_string());
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

    /// Returns just the parent segments (used for moving/deleting/adding files).
    pub fn parents(&self) -> &[&'a str] {
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

    pub async fn get_file(&self, file_path: &String) -> Result<Option<String>, FileServerError> {
        let Ok(path) = VirtualPath::parse(file_path, true) else {
            return Ok(None);
        };
        let mut con = self.connection().await?;
        let file: Option<File> =
            read_json_path(&mut con, &self.key, &file_json_path(path.path())).await?;
        Ok(file.map(|file| file.file_server))
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
    let kind: Value = redis::cmd("JSON.TYPE")
        .arg(key)
        .arg(path)
        .query_async(con)
        .await
        .map_err(|error| FileServerError::Redis(error.to_string()))?;
    if !json_type_exists(&kind) {
        return Ok(None);
    }

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

fn json_type_exists(value: &Value) -> bool {
    match value {
        Value::Nil => false,
        Value::Array(values) | Value::Set(values) => values.iter().any(json_type_exists),
        Value::Boolean(value) => *value,
        _ => true,
    }
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

fn file_json_path(components: &[&str]) -> String {
    let mut path = json_directory_path(&components[..components.len() - 1]);
    path.push_str("[\"files\"][");
    path.push_str(
        &serde_json::to_string(components.last().expect("file path has a name"))
            .expect("serialize path component"),
    );
    path.push(']');
    path
}

fn virtual_path(components: &[&str]) -> String {
    if components.is_empty() {
        "/".to_string()
    } else {
        format!("/{}", components.join("/"))
    }
}

fn path_parts(path: &str) -> Vec<String> {
    path.split('/')
        .filter(|component| !component.is_empty())
        .map(str::to_string)
        .collect()
}

fn json_directory_path_owned(components: &[String]) -> String {
    let refs = components.iter().map(String::as_str).collect::<Vec<_>>();
    json_directory_path(&refs)
}

fn json_file_path_owned(components: &[String]) -> String {
    let refs = components.iter().map(String::as_str).collect::<Vec<_>>();
    file_json_path(&refs)
}

fn child_virtual_path(parent: &str, name: &str) -> String {
    if parent == "/" {
        format!("/{name}")
    } else {
        format!("{parent}/{name}")
    }
}

fn parent_virtual_path(path: &str) -> &str {
    path.rsplit_once('/')
        .map(|(parent, _)| if parent.is_empty() { "/" } else { parent })
        .unwrap_or("/")
}

fn path_name(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or("")
}

impl FileTransaction {
    pub async fn new(
        path: PathBuf,
        key: String,
        mut con: MultiplexedConnection,
    ) -> Result<Self, FileServerError> {
        redis::cmd("WATCH")
            .arg(&key)
            .query_async::<()>(&mut con)
            .await
            .map_err(|error| FileServerError::Redis(error.to_string()))?;

        Ok(FileTransaction {
            path,
            key,
            con,
            mutations: Vec::new(),
            directory_overrides: HashMap::new(),
            file_overrides: HashMap::new(),
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
    pub async fn add_file(&mut self, file_path: String, content: Vec<u8>) -> Result<File, String> {
        self.ensure_open()?;
        info_span!("Adding file", file_path);
        let path = VirtualPath::parse(&file_path, true)?;
        self.create_up_to_dir(path.parents()).await?;

        let normalized_path = path.to_string_path();
        if self
            .lookup_file(&normalized_path)
            .await
            .map_err(|error| error.to_string())?
            .is_some()
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
        match std::fs::File::create(&disk_path) {
            Ok(mut disk_file) => {
                if let Err(error) = disk_file.write_all(&content) {
                    let _ = std::fs::remove_file(&disk_path);
                    return Err(error.to_string());
                }
            }
            Err(error) => return Err(error.to_string()),
        }
        self.created_files.push(disk_path);

        let parts = path_parts(&normalized_path);
        let json_path = json_file_path_owned(&parts);
        self.stage_set(&json_path, &file)
            .map_err(|error| error.to_string())?;
        self.file_overrides
            .insert(normalized_path, Some(file.clone()));
        Ok(file)
    }

    pub fn get_file_in(root: &Directory, file_path: &String) -> Option<String> {
        let path = VirtualPath::parse(file_path, true).ok()?;
        let mut directory = root;
        for parent in path.parents() {
            directory = directory.children.get(*parent)?;
        }
        directory
            .files
            .get(path.name())
            .map(|file| file.file_server.clone())
    }

    pub async fn move_file(
        &mut self,
        old_path: &String,
        new_path: &String,
    ) -> Result<File, String> {
        self.ensure_open()?;
        info_span!("Moving file", old_path, new_path);
        let old = VirtualPath::parse(old_path, true)?;
        let new = VirtualPath::parse(new_path, true)?;
        let old_normalized = old.to_string_path();
        let new_normalized = new.to_string_path();

        if !self
            .directory_exists(&virtual_path(old.parents()))
            .await
            .map_err(|error| error.to_string())?
        {
            return Err("Directory does not exist".into());
        }
        if !self
            .directory_exists(&virtual_path(new.parents()))
            .await
            .map_err(|error| error.to_string())?
        {
            return Err("Directory does not exist".into());
        }

        let mut file = self
            .lookup_file(&old_normalized)
            .await
            .map_err(|error| error.to_string())?
            .ok_or_else(|| format!("File {} does not exist", old_normalized))?;
        if self
            .lookup_file(&new_normalized)
            .await
            .map_err(|error| error.to_string())?
            .is_some()
        {
            return Err(format!(
                "Cannot move file since {} already exists",
                new_normalized
            ));
        }

        file.name = new.name().to_string();
        file.path = new_normalized.clone();
        self.stage_set(&json_file_path_owned(&path_parts(&new_normalized)), &file)
            .map_err(|error| error.to_string())?;
        self.stage_delete(&json_file_path_owned(&path_parts(&old_normalized)));
        self.file_overrides.insert(old_normalized, None);
        self.file_overrides
            .insert(new_normalized, Some(file.clone()));
        Ok(file)
    }

    pub async fn delete_file(&mut self, file_path: String) -> Result<File, String> {
        self.ensure_open()?;
        let path = VirtualPath::parse(&file_path, true)?;
        let normalized_path = path.to_string_path();
        let file = self
            .lookup_file(&normalized_path)
            .await
            .map_err(|error| error.to_string())?
            .ok_or_else(|| format!("File {} does not exists", normalized_path))?;

        self.stage_delete(&json_file_path_owned(&path_parts(&normalized_path)));
        self.file_overrides.insert(normalized_path, None);
        self.deleted_files.push(self.path.join(&file.file_server));
        Ok(file)
    }

    pub async fn add_dir(&mut self, dir_path: &String) -> Result<(), String> {
        self.ensure_open()?;
        info_span!("Adding directory", dir_path);
        let path = VirtualPath::parse(dir_path, false)?;
        self.create_up_to_dir(path.path()).await
    }

    pub async fn move_dir(
        &mut self,
        old_path: &String,
        new_path: &String,
    ) -> Result<Directory, String> {
        self.ensure_open()?;
        info_span!("Moving directory", old_path, new_path);
        let old = VirtualPath::parse(old_path, false)?;
        let new = VirtualPath::parse(new_path, false)?;

        if old.path().is_empty() {
            return Err("Cannot move root folder".into());
        }
        let old_normalized = virtual_path(old.path());
        let new_normalized = virtual_path(new.path());
        if old_normalized == new_normalized
            || new_normalized.starts_with(&format!("{old_normalized}/"))
        {
            error_span!("Cannot move directory into itself");
            return Err("Cannot move a directory into itself or its subdirectories.".into());
        }
        if !self
            .directory_exists(&virtual_path(old.parents()))
            .await
            .map_err(|error| error.to_string())?
            || !self
                .directory_exists(&virtual_path(new.parents()))
                .await
                .map_err(|error| error.to_string())?
        {
            return Err("Directory does not exist".into());
        }

        let mut directory = self
            .load_directory(&old_normalized)
            .await
            .map_err(|error| error.to_string())?
            .ok_or_else(|| format!("Directory {} does not exist", old_normalized))?;
        if self
            .directory_exists(&new_normalized)
            .await
            .map_err(|error| error.to_string())?
        {
            return Err(format!(
                "Cannot move directory since {} already exists",
                new_normalized
            ));
        }

        directory.name = new.name().to_string();
        update_directory_path(&mut directory, &new_normalized);
        self.stage_set(
            &json_directory_path_owned(&path_parts(&new_normalized)),
            &directory,
        )
        .map_err(|error| error.to_string())?;
        self.stage_delete(&json_directory_path_owned(&path_parts(&old_normalized)));

        self.clear_overrides_under(&old_normalized);
        self.directory_overrides.insert(old_normalized, None);
        self.directory_overrides
            .insert(new_normalized, Some(directory.clone()));
        Ok(directory)
    }

    pub async fn delete_dir(&mut self, dir_path: String) -> Result<Directory, String> {
        self.ensure_open()?;
        info_span!("Deleting directory", dir_path);
        let path = VirtualPath::parse(&dir_path, false)?;
        if path.path().is_empty() {
            return Err("Will not delete root folder".into());
        }
        let normalized_path = virtual_path(path.path());
        let directory = self
            .load_directory(&normalized_path)
            .await
            .map_err(|error| error.to_string())?
            .ok_or_else(|| "Directory does not exist".to_string())?;

        let mut files = Vec::new();
        collect_files(&directory, &mut files);
        self.deleted_files.extend(
            files
                .into_iter()
                .map(|file| self.path.join(file.file_server)),
        );
        self.stage_delete(&json_directory_path_owned(&path_parts(&normalized_path)));
        self.clear_overrides_under(&normalized_path);
        self.directory_overrides.insert(normalized_path, None);
        Ok(directory)
    }

    async fn create_up_to_dir(&mut self, components: &[&str]) -> Result<(), String> {
        let mut current = Vec::<String>::new();
        for component in components {
            current.push((*component).to_string());
            let path = virtual_path(&current.iter().map(String::as_str).collect::<Vec<_>>());
            if !self
                .directory_exists(&path)
                .await
                .map_err(|error| error.to_string())?
            {
                let directory = Directory {
                    name: (*component).to_string(),
                    path: path.clone(),
                    files: HashMap::new(),
                    children: HashMap::new(),
                };
                self.stage_set(&json_directory_path_owned(&current), &directory)
                    .map_err(|error| error.to_string())?;
                self.directory_overrides.insert(path, Some(directory));
            }
        }
        Ok(())
    }

    async fn directory_exists(&mut self, path: &str) -> Result<bool, FileServerError> {
        if let Some(directory) = self.directory_from_overrides(path) {
            return Ok(directory.is_some());
        }
        let mut json_path = json_directory_path_owned(&path_parts(path));
        json_path.push_str("[\"name\"]");
        read_json_path::<String>(&mut self.con, &self.key, &json_path)
            .await
            .map(|value| value.is_some())
    }

    async fn lookup_file(&mut self, path: &str) -> Result<Option<File>, FileServerError> {
        if let Some(file) = self.file_overrides.get(path) {
            return Ok(file.clone());
        }

        let parent = parent_virtual_path(path);
        if let Some(directory) = self.directory_from_overrides(parent) {
            let Some(directory) = directory else {
                return Ok(None);
            };
            return Ok(directory.files.get(path_name(path)).cloned());
        }

        read_json_path::<File>(
            &mut self.con,
            &self.key,
            &json_file_path_owned(&path_parts(path)),
        )
        .await
    }

    async fn load_directory(&mut self, path: &str) -> Result<Option<Directory>, FileServerError> {
        let mut directory = if let Some(directory) = self.directory_from_overrides(path) {
            directory
        } else {
            read_json_path::<Directory>(
                &mut self.con,
                &self.key,
                &json_directory_path_owned(&path_parts(path)),
            )
            .await?
        };
        if let Some(directory) = directory.as_mut() {
            self.apply_overrides(directory, path);
        }
        Ok(directory)
    }

    fn directory_from_overrides(&self, path: &str) -> Option<Option<Directory>> {
        let parts = path_parts(path);
        for prefix_len in (0..=parts.len()).rev() {
            let prefix = virtual_path(
                &parts[..prefix_len]
                    .iter()
                    .map(String::as_str)
                    .collect::<Vec<_>>(),
            );
            let Some(staged) = self.directory_overrides.get(&prefix) else {
                continue;
            };
            let Some(mut directory) = staged.clone() else {
                return Some(None);
            };
            self.apply_overrides(&mut directory, &prefix);
            let mut current_path = prefix;
            for component in &parts[prefix_len..] {
                let Some(child) = directory.children.get(component).cloned() else {
                    return Some(None);
                };
                directory = child;
                current_path = child_virtual_path(&current_path, component);
                self.apply_overrides(&mut directory, &current_path);
            }
            return Some(Some(directory));
        }
        None
    }

    fn apply_overrides(&self, directory: &mut Directory, path: &str) {
        let directory_changes = self
            .directory_overrides
            .iter()
            .filter(|(child_path, _)| parent_virtual_path(child_path) == path)
            .map(|(child_path, value)| (path_name(child_path).to_string(), value.clone()))
            .collect::<Vec<_>>();
        for (name, value) in directory_changes {
            match value {
                Some(child) => {
                    directory.children.insert(name, child);
                }
                None => {
                    directory.children.remove(&name);
                }
            }
        }

        let file_changes = self
            .file_overrides
            .iter()
            .filter(|(file_path, _)| parent_virtual_path(file_path) == path)
            .map(|(file_path, value)| (path_name(file_path).to_string(), value.clone()))
            .collect::<Vec<_>>();
        for (name, value) in file_changes {
            match value {
                Some(file) => {
                    directory.files.insert(name, file);
                }
                None => {
                    directory.files.remove(&name);
                }
            }
        }

        let children = directory
            .children
            .iter_mut()
            .map(|(name, child)| (name.clone(), child))
            .collect::<Vec<_>>();
        for (name, child) in children {
            self.apply_overrides(child, &child_virtual_path(path, &name));
        }
    }

    fn clear_overrides_under(&mut self, path: &str) {
        self.directory_overrides
            .retain(|key, _| key != path && !key.starts_with(&format!("{path}/")));
        self.file_overrides
            .retain(|key, _| !key.starts_with(&format!("{path}/")));
    }

    fn stage_set<T: Serialize>(&mut self, path: &str, value: &T) -> Result<(), FileServerError> {
        let value = serde_json::to_string(value)
            .map_err(|error| FileServerError::InvalidData(error.to_string()))?;
        self.mutations.push(Mutation::Set {
            path: path.to_string(),
            value,
        });
        Ok(())
    }

    fn stage_delete(&mut self, path: &str) {
        self.mutations.push(Mutation::Delete {
            path: path.to_string(),
        });
    }

    pub async fn abort_transaction(&mut self) {
        if self.finished {
            return;
        }
        self.finished = true;
        let _ = redis::cmd("UNWATCH").query_async::<()>(&mut self.con).await;
        self.cleanup_created_files().await;
        self.deleted_files.clear();
        self.mutations.clear();
    }

    pub async fn write(&mut self) -> Result<(), FileServerError> {
        //TODO: build local tree, apply mutations and commit everything in
        // one `JSON.SET`. Avoids individual `JSON.SET` calls which may fail
        // individually but *probably* should succeed or fail together.
        // The simplicity costs having to load the entire tree into memory.
        // Worth it?
        if self.finished {
            return Err(FileServerError::TransactionClosed);
        }
        self.finished = true;

        if let Err(error) = redis::cmd("MULTI")
            .query_async::<Value>(&mut self.con)
            .await
        {
            self.cleanup_created_files().await;
            self.deleted_files.clear();
            return Err(FileServerError::Redis(error.to_string()));
        }

        for mutation in &self.mutations {
            let result = match mutation {
                Mutation::Set { path, value } => {
                    redis::cmd("JSON.SET")
                        .arg(&self.key)
                        .arg(path)
                        .arg(value)
                        .query_async::<Value>(&mut self.con)
                        .await
                }
                Mutation::Delete { path } => {
                    redis::cmd("JSON.DEL")
                        .arg(&self.key)
                        .arg(path)
                        .query_async::<Value>(&mut self.con)
                        .await
                }
            };
            if let Err(error) = result {
                let _ = redis::cmd("DISCARD")
                    .query_async::<Value>(&mut self.con)
                    .await;
                self.cleanup_created_files().await;
                self.deleted_files.clear();
                return Err(FileServerError::Redis(error.to_string()));
            }
        }

        let result: Option<Vec<Value>> = match redis::cmd("EXEC").query_async(&mut self.con).await {
            Ok(result) => result,
            Err(error) => {
                // The request may have reached Redis and committed despite a lost or
                // malformed response. Keep the files to avoid dangling committed metadata.
                self.created_files.clear();
                self.deleted_files.clear();
                return Err(FileServerError::Redis(error.to_string()));
            }
        };

        match result {
            Some(results) => {
                let result_count_mismatch = results.len() != self.mutations.len();
                let command_error = self
                    .mutations
                    .iter()
                    .zip(&results)
                    .find_map(|(mutation, result)| match (mutation, result) {
                        (_, Value::ServerError(error)) => Some(error.to_string()),
                        (Mutation::Set { path, .. }, Value::Nil) => {
                            Some(format!("Redis JSON.SET did not update {path}"))
                        }
                        (Mutation::Delete { path }, Value::Int(0) | Value::Nil) => {
                            Some(format!("Redis JSON.DEL did not delete {path}"))
                        }
                        _ => None,
                    })
                    .or_else(|| {
                        result_count_mismatch.then(|| {
                            format!(
                                "Redis EXEC returned {} results for {} mutations",
                                results.len(),
                                self.mutations.len()
                            )
                        })
                    });

                if let Some(error) = command_error {
                    // TODO: list individual mutations that failed to log/return them.
                    let may_have_applied_mutation = result_count_mismatch
                        || self
                            .mutations
                            .iter()
                            .zip(&results)
                            .any(|(mutation, result)| match mutation {
                                Mutation::Set { .. } => {
                                    !matches!(result, Value::ServerError(_) | Value::Nil)
                                }
                                Mutation::Delete { .. } => !matches!(
                                    result,
                                    Value::ServerError(_) | Value::Nil | Value::Int(0)
                                ),
                            });
                    if may_have_applied_mutation {
                        // EXEC does not roll back successful commands when another command
                        // fails. Preserve possible file references; orphan bytes are safer.
                        self.created_files.clear();
                    } else {
                        self.cleanup_created_files().await;
                    }
                    self.deleted_files.clear();
                    return Err(FileServerError::Redis(format!(
                        "Redis transaction command failed: {error}"
                    )));
                }

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

fn update_directory_path(directory: &mut Directory, path: &str) {
    directory.path = path.to_string();
    let child_names = directory.children.keys().cloned().collect::<Vec<_>>();
    for name in child_names {
        if let Some(child) = directory.children.get_mut(&name) {
            update_directory_path(child, &child_virtual_path(path, &name));
        }
    }
    for file in directory.files.values_mut() {
        file.path = child_virtual_path(path, &file.name);
    }
}

fn collect_files(directory: &Directory, files: &mut Vec<File>) {
    files.extend(directory.files.values().cloned());
    let mut children = directory.children.values().collect::<Vec<_>>();
    children.sort_by(|left, right| left.name.cmp(&right.name));
    for child in children {
        collect_files(child, files);
    }
}
