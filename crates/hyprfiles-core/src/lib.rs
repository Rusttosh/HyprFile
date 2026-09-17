pub mod config;
pub mod events;
pub mod fs;
pub mod jobs;
pub mod mime;
pub mod preview;
pub mod search;
pub mod theme;
pub mod trash;

use std::path::PathBuf;

/// Represents a single file system entry (file, directory, or symlink).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Entry {
    pub path: PathBuf,
    pub name: String,
    pub entry_type: EntryType,
    pub size: u64,
    pub modified: Option<std::time::SystemTime>,
    pub permissions: Option<u32>,
    pub is_hidden: bool,
    pub is_symlink: bool,
    pub symlink_target: Option<PathBuf>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum EntryType {
    File,
    Directory,
    Symlink,
    Unknown,
}

/// Sort criteria for directory listings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum SortBy {
    Name,
    NameInsensitive,
    Size,
    Modified,
    Extension,
    Type,
}

/// Direction for sorting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum SortDir {
    Ascending,
    Descending,
}

/// Request to list a directory.
#[derive(Debug, Clone)]
pub struct ListDirRequest {
    pub path: PathBuf,
    pub show_hidden: bool,
    pub sort_by: SortBy,
    pub sort_dir: SortDir,
    pub filter: Option<String>,
}

/// Result of a directory listing.
#[derive(Debug, Clone)]
pub struct ListDirResult {
    pub path: PathBuf,
    pub entries: Vec<Entry>,
    pub parent: Option<PathBuf>,
}


