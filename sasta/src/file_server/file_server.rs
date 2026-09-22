use std::{
    collections::VecDeque,
    ffi::OsString,
    io::Write,
    path::{Path, PathBuf},
    sync::{Arc, LazyLock, Mutex},
};

use chrono::{DateTime, Local};
#[cfg(not(test))]
use redis::{Client, JsonAsyncCommands, aio::MultiplexedConnection};
use regex::Regex;
use serde::{Deserialize, Serialize};
use tokio::fs;
#[cfg(not(test))]
use tokio::sync::Mutex as AsyncMutex;
use tracing::warn;
use tracing::{error_span, info_span};
use uuid::Uuid;

// #[derive(Serialize, ToSchema)]
// #[serde(tag = "type", content = "content")]
// pub enum Payload {
//     FilePaths(ListView),
//     Error { code: u8, message: String },
// }

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct File {
    pub name: String,
    /// Actual filename on disk
    ///
    /// `{UUID}.{ext}`
    pub file_server: String,
    /// File path through built file tree
    pub path: String,
    pub size: usize,
    pub date: DateTime<Local>,
}

#[derive(Clone, Debug)]
pub struct Directory {
    pub name: String,
    pub path: String,
    pub files: Arc<Mutex<Vec<File>>>,
    pub children: Arc<Mutex<Vec<Directory>>>,
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
    pub root: Directory,
    pub path: PathBuf,
    #[cfg(not(test))]
    con: AsyncMutex<MultiplexedConnection>,
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

    /// Returns the full path as an array slice
    pub fn path(&self) -> &[&'a str] {
        match self {
            VirtualPath::Root => &[],
            VirtualPath::File(c) | VirtualPath::Directory(c) => c,
        }
    }

    /// Returns just the parent segments (Used for moving/deleting/adding files)
    pub fn parents(&self) -> &[&'a str] {
        match self {
            VirtualPath::Root => &[],
            VirtualPath::File(c) | VirtualPath::Directory(c) => {
                if c.is_empty() { &[] } else { &c[..c.len() - 1] } // Fast sub-slicing!
            }
        }
    }

    pub fn name(&self) -> &'a str {
        match self {
            VirtualPath::Root => "root",
            VirtualPath::File(c) | VirtualPath::Directory(c) => c.last().unwrap_or(&"root"),
        }
    }

    /// Reconstructs a clean String path
    pub fn to_string_path(&self) -> String {
        match self {
            VirtualPath::Root => "/".to_string(),
            VirtualPath::File(c) | VirtualPath::Directory(c) => format!("/{}", c.join("/")),
        }
    }
}

impl FileServer {
    #[cfg(not(test))]
    pub async fn new(redis_url: &str, path: impl Into<PathBuf>) -> Self {
        let path = path.into();
        fs::create_dir_all(&path).await.unwrap();
        let client = Client::open(redis_url).unwrap();
        let mut con = client.get_multiplexed_async_connection().await.unwrap();

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
            path,
        }
    }

    #[cfg(test)]
    pub async fn new(_redis_url: &str, path: impl Into<PathBuf>) -> Self {
        let path = path.into();
        fs::create_dir_all(&path).await.unwrap();

        // Return a fresh, empty in-memory tree for every test
        Self {
            root: Directory {
                name: "".to_string(),
                path: String::from("/"),
                files: Arc::new(Mutex::new(vec![])),
                children: Arc::new(Mutex::new(vec![])),
            },
            path,
        }
    }

    /// Add file name to directory tree, and create folder if they don't already exists
    ///
    /// Does not call write to avoid writing when not all files are returning Ok()
    pub async fn add_file(&mut self, file_path: String, content: Vec<u8>) -> Result<File, String> {
        info_span!("Adding file with ", file_path);
        let path = VirtualPath::parse(&file_path, true)?;

        let dir = self.create_up_to_dir(path.parents());

        let mut files = dir.files.lock().unwrap();
        match files.binary_search_by_key(&path.name(), |f| &f.name) {
            Ok(_) => Err(format!("File {} already exists", path.to_string_path())),
            Err(pos) => {
                let file = File {
                    name: path.name().to_string(),
                    file_server: format!(
                        "{}.{}",
                        Uuid::new_v4(),
                        Path::new(path.name())
                            .extension()
                            .unwrap_or(&OsString::from("txt"))
                            .to_str()
                            .unwrap()
                    ),
                    path: path.to_string_path(),
                    size: content.len(),
                    date: Local::now(),
                };

                match std::fs::File::create(self.path.join(&file.file_server)) {
                    Ok(mut file) => {
                        if let Err(e) = file.write_all(&content) {
                            return Err(e.to_string());
                        }
                    }
                    Err(e) => return Err(e.to_string()),
                };

                files.insert(pos, file.clone());
                Ok(file)
            }
        }
    }

    pub async fn get_file(&self, file_path: &String) -> Option<String> {
        let path = VirtualPath::parse(file_path, true).ok()?;

        let dir = self.traverse_to_dir(path.parents())?;

        let files = dir.files.lock().unwrap();
        match files.binary_search_by_key(&path.name(), |f| &f.name) {
            Ok(pos) => Some(files[pos].file_server.clone()),
            Err(_) => None,
        }
    }

    pub async fn move_file(&self, old_path: &String, new_path: &String) -> Result<File, String> {
        info_span!("Moving file", old_path, new_path);

        let old_path = VirtualPath::parse(old_path, true)?;
        let new_path = VirtualPath::parse(new_path, true)?;

        let old_dir = match self.traverse_to_dir(old_path.parents()) {
            Some(d) => d,
            None => return Err(String::from("Directory does not exist")),
        };

        let new_dir = match self.traverse_to_dir(new_path.parents()) {
            Some(d) => d,
            None => return Err(String::from("Directory does not exist")),
        };

        let mut old_dir_files = old_dir.files.lock().unwrap();
        let old_dir_file_pos =
            match old_dir_files.binary_search_by_key(&old_path.name(), |f| &f.name) {
                Ok(pos) => pos,
                Err(_) => return Err(format!("File {} does not exist", old_path.to_string_path())),
            };

        if new_dir.path == old_dir.path {
            match old_dir_files.binary_search_by_key(&new_path.name(), |f| &f.name) {
                Ok(_) => Err(format!(
                    "Cannot move file since {} already exists",
                    new_path.to_string_path()
                )),
                Err(pos) => {
                    let mut file = old_dir_files.remove(old_dir_file_pos);
                    file.path = new_path.to_string_path();
                    file.name = new_path.name().to_string();

                    // Decrease insert pos by on if file will be inserted after the before position
                    // to account for itself being removed earlier in the array
                    let insert_pos = if old_dir_file_pos < pos { pos - 1 } else { pos };
                    old_dir_files.insert(insert_pos, file.clone());

                    Ok(file)
                }
            }
        } else {
            let mut move_to_dir_files = new_dir.files.lock().unwrap();
            match move_to_dir_files.binary_search_by_key(&new_path.name(), |f| &f.name) {
                Ok(_) => Err(format!(
                    "Cannot move file since {} already exists",
                    new_path.to_string_path()
                )),
                Err(pos) => {
                    let mut file = old_dir_files.remove(old_dir_file_pos);
                    file.path = new_path.to_string_path();
                    file.name = new_path.name().to_string();

                    move_to_dir_files.insert(pos, file.clone());

                    Ok(file)
                }
            }
        }
    }

    pub async fn delete_file(&mut self, file_path: String) -> Result<File, String> {
        let path = VirtualPath::parse(&file_path, true)?;

        let dir = match self.traverse_to_dir(path.parents()) {
            Some(d) => d,
            None => return Err(String::from("Directory does not exist")),
        };

        let file = {
            let mut files = dir.files.lock().unwrap();
            match files.binary_search_by_key(&path.name(), |f| &f.name) {
                Ok(pos) => files.remove(pos),
                Err(_) => return Err(format!("File {} does not exists", path.to_string_path())),
            }
        };

        match fs::remove_file(self.path.join(&file.file_server)).await {
            Ok(_) => Ok(file),
            Err(e) => Err(e.to_string()),
        }
    }

    pub async fn add_dir(&mut self, dir_path: &String) -> Result<(), String> {
        info_span!("Adding dir ", dir_path);
        let path = VirtualPath::parse(dir_path, false)?;
        let _ = self.create_up_to_dir(path.path());
        Ok(())
    }

    pub async fn move_dir(
        &self,
        old_path: &String,
        new_path: &String,
    ) -> Result<Directory, String> {
        info_span!("Moving dir", old_path, new_path);

        let old_path = VirtualPath::parse(old_path, false)?;
        let new_path = VirtualPath::parse(new_path, false)?;

        if old_path.path().is_empty() {
            return Err(String::from("Cannot move root folder"));
        }

        if old_path == new_path
            || new_path
                .to_string_path()
                .starts_with(&format!("{}/", old_path.to_string_path()))
        {
            error_span!("Cannot move directory into itself");
            return Err("Cannot move a directory into itself or its subdirectories.".to_string());
        }

        let old_parent_dir = match self.traverse_to_dir(old_path.parents()) {
            Some(d) => d,
            None => {
                error_span!("Parent directory does not exist");
                return Err(String::from("Directory does not exist"));
            }
        };

        let new_parent_dir = match self.traverse_to_dir(new_path.parents()) {
            Some(d) => d,
            None => {
                error_span!("Parent directory does not exist");
                return Err(String::from("Directory does not exist"));
            }
        };

        let mut old_parent_dir_dirs = old_parent_dir.children.lock().unwrap();
        let old_parent_dir_pos =
            match old_parent_dir_dirs.binary_search_by_key(&old_path.name(), |d| &d.name) {
                Ok(pos) => pos,
                Err(_) => {
                    return Err(format!(
                        "Directory {} does not exist",
                        old_path.to_string_path()
                    ));
                }
            };

        let dir = if old_parent_dir.path == new_parent_dir.path {
            match old_parent_dir_dirs.binary_search_by_key(&new_path.name(), |d| &d.name) {
                Ok(_) => {
                    return Err(format!(
                        "Cannot move directory since {} already exists",
                        new_path.to_string_path()
                    ));
                }
                Err(pos) => {
                    let mut dir = old_parent_dir_dirs.remove(old_parent_dir_pos);
                    dir.path = new_path.to_string_path();
                    dir.name = new_path.name().to_string();

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
            let mut new_parent_dir_children = new_parent_dir.children.lock().unwrap();
            match new_parent_dir_children.binary_search_by_key(&new_path.name(), |f| &f.name) {
                Ok(_) => {
                    return Err(format!(
                        "Cannot move directory since {} already exists",
                        new_path.to_string_path()
                    ));
                }
                Err(pos) => {
                    let mut dir = old_parent_dir_dirs.remove(old_parent_dir_pos);
                    dir.path = new_path.to_string_path();
                    dir.name = new_path.name().to_string();
                    new_parent_dir_children.insert(pos, dir.clone());

                    dir
                }
            }
        };
        Self::recursively_update_path(&dir);
        Ok(dir)
    }

    fn recursively_update_path(dir: &Directory) {
        for d in dir.children.lock().unwrap().iter_mut() {
            d.path = format!("{}/{}", dir.path, d.name);
            Self::recursively_update_path(d);
        }
        for f in dir.files.lock().unwrap().iter_mut() {
            f.path = format!("{}/{}", dir.path, f.name)
        }
    }

    pub async fn delete_dir(&mut self, dir_path: String) -> Result<Directory, String> {
        info_span!("Deleting dir", dir_path);
        let path = VirtualPath::parse(&dir_path, false)?;

        if path.path().is_empty() {
            return Err(String::from("Will not delete root folder"));
        }

        let parent_dir = match self.traverse_to_dir(path.parents()) {
            Some(d) => d,
            None => {
                error_span!("Parent directory does not exist");
                return Err(String::from("Parent directory does not exist"));
            }
        };
        let dir = {
            let mut dirs = parent_dir.children.lock().unwrap();
            match dirs.binary_search_by_key(&path.name(), |d| &d.name) {
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
            if let Err(e) = fs::remove_file(self.path.join(&f.file_server)).await {
                warn!("Error deleting file {:?}", e);
            }
        }
        Ok(dir)
    }

    /// Traverse through tree until path and create dirs on the way
    fn create_up_to_dir(&self, path: &[&str]) -> Directory {
        let mut dir = self.root.clone();
        let mut current_path = String::new();

        for p in path {
            current_path.push('/');
            current_path.push_str(p);

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
                            path: current_path.clone(),
                            files: Arc::new(Mutex::new(vec![])),
                            children: Arc::new(Mutex::new(vec![])),
                        },
                    );
                    pos
                }
            };
            dir = d.get(pos).unwrap().clone();
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
    pub async fn write(&mut self) {
        // Do not write to any db when in test environment
    }

    #[cfg(not(test))]
    pub async fn write(&mut self) {
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
