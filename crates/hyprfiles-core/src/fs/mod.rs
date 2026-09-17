use crate::{Entry, EntryType, ListDirRequest, ListDirResult, SortBy, SortDir};
use anyhow::{Context, Result};
use std::future::Future;
use std::path::PathBuf;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use thiserror::Error;
use tracing::{debug, warn};

/// Errors that can occur during filesystem operations.
#[derive(Debug, Error)]
pub enum FsError {
    #[error("permission denied: {0}")]
    PermissionDenied(PathBuf),
    #[error("path not found: {0}")]
    NotFound(PathBuf),
    #[error("not a directory: {0}")]
    NotADirectory(PathBuf),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("invalid path: {0}")]
    InvalidPath(String),
}

/// Trait for filesystem providers.
pub trait FsProvider: Send + Sync {
    fn list_dir(
        &self,
        req: ListDirRequest,
    ) -> impl Future<Output = Result<ListDirResult>> + Send;
    fn metadata(&self, path: PathBuf) -> impl Future<Output = Result<Entry>> + Send;
    fn exists(&self, path: PathBuf) -> impl Future<Output = Result<bool>> + Send;
}

/// Local filesystem provider.
#[derive(Debug, Clone)]
pub struct LocalFsProvider {
    pub follow_symlinks: bool,
}

impl Default for LocalFsProvider {
    fn default() -> Self {
        Self {
            follow_symlinks: false,
        }
    }
}

impl LocalFsProvider {
    /// Create a new provider with the given symlink behavior.
    pub fn new(follow_symlinks: bool) -> Self {
        Self { follow_symlinks }
    }

    /// Determine whether a file is hidden (starts with `.` on Unix).
    fn is_hidden_name(name: &str) -> bool {
        name.starts_with('.')
    }

    /// Get the extension of a filename, or empty string.
    fn ext_of(name: &str) -> String {
        name.rfind('.')
            .map(|i| name[i + 1..].to_lowercase())
            .unwrap_or_default()
    }

    /// Build an [`Entry`] from a directory entry, respecting `follow_symlinks`.
    async fn entry_from_direntry(&self, dirent: &tokio::fs::DirEntry) -> Result<Entry> {
        let path = dirent.path();
        let name = dirent
            .file_name()
            .into_string()
            .unwrap_or_else(|_| path.to_string_lossy().into_owned());

        let mut is_symlink = false;
        let mut symlink_target = None;

        // Use symlink_metadata unless follow_symlinks is true
        let meta = if self.follow_symlinks {
            match dirent.metadata().await {
                Ok(m) => m,
                Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => {
                    return Err(FsError::PermissionDenied(path.clone()).into());
                }
                Err(e) => return Err(FsError::Io(e).into()),
            }
        } else {
            match dirent.metadata().await {
                Ok(m) => {
                    // Check if it's a symlink by trying symlink_metadata
                    if let Ok(sym) = tokio::fs::symlink_metadata(&path).await {
                        if sym.file_type().is_symlink() {
                            is_symlink = true;
                            symlink_target = tokio::fs::read_link(&path).await.ok();
                        }
                    }
                    m
                }
                Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => {
                    return Err(FsError::PermissionDenied(path.clone()).into());
                }
                Err(e) => return Err(FsError::Io(e).into()),
            }
        };

        let file_type = meta.file_type();
        let entry_type = if file_type.is_symlink() {
            is_symlink = true;
            if symlink_target.is_none() {
                symlink_target = tokio::fs::read_link(&path).await.ok();
            }
            EntryType::Symlink
        } else if file_type.is_dir() {
            EntryType::Directory
        } else if file_type.is_file() {
            EntryType::File
        } else {
            EntryType::Unknown
        };

        Ok(Entry {
            path: path.clone(),
            name: name.clone(),
            entry_type,
            size: meta.len(),
            modified: meta.modified().ok(),
            permissions: Some(meta.permissions().mode()),
            is_hidden: Self::is_hidden_name(&name),
            is_symlink,
            symlink_target,
        })
    }

    /// Build an [`Entry`] from an arbitrary path.
    async fn entry_from_path(&self, path: PathBuf) -> Result<Entry> {
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| path.to_string_lossy().into_owned());

        let meta = if self.follow_symlinks {
            match tokio::fs::metadata(&path).await {
                Ok(m) => m,
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                    return Err(FsError::NotFound(path).into());
                }
                Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => {
                    return Err(FsError::PermissionDenied(path).into());
                }
                Err(e) => return Err(FsError::Io(e).into()),
            }
        } else {
            let sym = tokio::fs::symlink_metadata(&path).await.map_err(|e| {
                if e.kind() == std::io::ErrorKind::NotFound {
                    FsError::NotFound(path.clone())
                } else if e.kind() == std::io::ErrorKind::PermissionDenied {
                    FsError::PermissionDenied(path.clone())
                } else {
                    FsError::Io(e)
                }
            })?;
            sym
        };

        let file_type = meta.file_type();
        let is_symlink = file_type.is_symlink();
        let symlink_target = if is_symlink {
            tokio::fs::read_link(&path).await.ok()
        } else {
            None
        };

        let entry_type = if is_symlink {
            EntryType::Symlink
        } else if file_type.is_dir() {
            EntryType::Directory
        } else if file_type.is_file() {
            EntryType::File
        } else {
            EntryType::Unknown
        };

        Ok(Entry {
            path: path.clone(),
            name: name.clone(),
            entry_type,
            size: meta.len(),
            modified: meta.modified().ok(),
            permissions: Some(meta.permissions().mode()),
            is_hidden: Self::is_hidden_name(&name),
            is_symlink,
            symlink_target,
        })
    }

    /// Filter entries using a basic glob/fuzzy string match.
    fn apply_filter(entries: Vec<Entry>, filter: &str) -> Vec<Entry> {
        let lower = filter.to_lowercase();
        entries
            .into_iter()
            .filter(|e| e.name.to_lowercase().contains(&lower))
            .collect()
    }

    /// Sort entries according to the request. Falls back to name for tie-breaking.
    fn apply_sort(entries: &mut [Entry], sort_by: SortBy, sort_dir: SortDir) {
        let primary: Box<dyn Fn(&Entry, &Entry) -> std::cmp::Ordering> = match sort_by {
            SortBy::Name => Box::new(|a, b| a.name.cmp(&b.name)),
            SortBy::NameInsensitive => Box::new(|a, b| {
                a.name.to_lowercase().cmp(&b.name.to_lowercase())
            }),
            SortBy::Size => Box::new(|a, b| a.size.cmp(&b.size)),
            SortBy::Modified => Box::new(|a, b| {
                a.modified.unwrap_or(std::time::UNIX_EPOCH)
                    .cmp(&b.modified.unwrap_or(std::time::UNIX_EPOCH))
            }),
            SortBy::Extension => Box::new(|a, b| {
                Self::ext_of(&a.name).cmp(&Self::ext_of(&b.name))
            }),
            SortBy::Type => Box::new(|a, b| {
                let order = |t: EntryType| match t {
                    EntryType::Directory => 0,
                    EntryType::Symlink => 1,
                    EntryType::File => 2,
                    EntryType::Unknown => 3,
                };
                order(a.entry_type).cmp(&order(b.entry_type))
            }),
        };

        entries.sort_by(|a, b| {
            let ord = primary(a, b).then_with(|| a.name.cmp(&b.name));
            if sort_dir == SortDir::Descending {
                ord.reverse()
            } else {
                ord
            }
        });
    }
}

impl FsProvider for LocalFsProvider {
    async fn list_dir(&self, req: ListDirRequest) -> Result<ListDirResult> {
        let path = &req.path;
        debug!("listing directory {}", path.display());

        let mut entries = Vec::new();
        let mut read_dir = match tokio::fs::read_dir(path).await {
            Ok(rd) => rd,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return Err(FsError::NotFound(path.clone()).into());
            }
            Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => {
                return Err(FsError::PermissionDenied(path.clone()).into());
            }
            Err(e) => return Err(FsError::Io(e).into()),
        };

        while let Some(dirent) = read_dir.next_entry().await? {
            let name = dirent.file_name();
            let name_str = name.to_string_lossy();
            let is_hidden = Self::is_hidden_name(&name_str);

            if !req.show_hidden && is_hidden {
                continue;
            }

            match self.entry_from_direntry(&dirent).await {
                Ok(entry) => entries.push(entry),
                Err(e) => {
                    warn!("failed to read entry {}: {}", dirent.path().display(), e);
                }
            }
        }

        // sort
        Self::apply_sort(&mut entries, req.sort_by, req.sort_dir);

        // filter
        let entries = if let Some(filter) = &req.filter {
            Self::apply_filter(entries, filter)
        } else {
            entries
        };

        let parent = path.parent().map(|p| p.to_path_buf());
        Ok(ListDirResult {
            path: path.clone(),
            entries,
            parent,
        })
    }

    async fn metadata(&self, path: PathBuf) -> Result<Entry> {
        if !path.exists() {
            return Err(FsError::NotFound(path).into());
        }
        self.entry_from_path(path).await
    }

    async fn exists(&self, path: PathBuf) -> Result<bool> {
        Ok(tokio::fs::try_exists(&path).await.unwrap_or(false))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::SystemTime;
    use tempfile::TempDir;

    fn provider() -> LocalFsProvider {
        LocalFsProvider::default()
    }

    #[tokio::test]
    async fn test_list_dir_basic() {
        let tmp = TempDir::new().unwrap();
        let base = tmp.path();
        tokio::fs::write(base.join("a.txt"), "hello").await.unwrap();
        tokio::fs::write(base.join("b.txt"), "world").await.unwrap();
        tokio::fs::create_dir(base.join("c_dir")).await.unwrap();

        let req = ListDirRequest {
            path: base.to_path_buf(),
            show_hidden: true,
            sort_by: SortBy::Name,
            sort_dir: SortDir::Ascending,
            filter: None,
        };
        let res = provider().list_dir(req).await.unwrap();
        assert_eq!(res.entries.len(), 3);
        let names: Vec<_> = res.entries.iter().map(|e| e.name.clone()).collect();
        assert_eq!(names, vec!["a.txt", "b.txt", "c_dir"]);
    }

    #[tokio::test]
    async fn test_list_dir_hidden() {
        let tmp = TempDir::new().unwrap();
        let base = tmp.path();
        tokio::fs::write(base.join("visible"), "v").await.unwrap();
        tokio::fs::write(base.join(".hidden"), "h").await.unwrap();

        let req = ListDirRequest {
            path: base.to_path_buf(),
            show_hidden: false,
            sort_by: SortBy::Name,
            sort_dir: SortDir::Ascending,
            filter: None,
        };
        let res = provider().list_dir(req).await.unwrap();
        assert_eq!(res.entries.len(), 1);
        assert_eq!(res.entries[0].name, "visible");
    }

    #[tokio::test]
    async fn test_list_dir_filter() {
        let tmp = TempDir::new().unwrap();
        let base = tmp.path();
        tokio::fs::write(base.join("foo.txt"), "").await.unwrap();
        tokio::fs::write(base.join("bar.log"), "").await.unwrap();
        tokio::fs::write(base.join("foobar.rs"), "").await.unwrap();

        let req = ListDirRequest {
            path: base.to_path_buf(),
            show_hidden: true,
            sort_by: SortBy::Name,
            sort_dir: SortDir::Ascending,
            filter: Some("foo".to_string()),
        };
        let res = provider().list_dir(req).await.unwrap();
        let names: Vec<_> = res.entries.iter().map(|e| e.name.clone()).collect();
        assert_eq!(names, vec!["foo.txt", "foobar.rs"]);
    }

    #[tokio::test]
    async fn test_sort_by_size() {
        let tmp = TempDir::new().unwrap();
        let base = tmp.path();
        tokio::fs::write(base.join("small"), "x").await.unwrap();
        tokio::fs::write(base.join("large"), "x".repeat(100)).await.unwrap();

        let req = ListDirRequest {
            path: base.to_path_buf(),
            show_hidden: true,
            sort_by: SortBy::Size,
            sort_dir: SortDir::Ascending,
            filter: None,
        };
        let res = provider().list_dir(req).await.unwrap();
        let names: Vec<_> = res.entries.iter().map(|e| e.name.clone()).collect();
        assert_eq!(names, vec!["small", "large"]);
    }

    #[tokio::test]
    async fn test_sort_by_extension() {
        let tmp = TempDir::new().unwrap();
        let base = tmp.path();
        tokio::fs::write(base.join("a.rs"), "").await.unwrap();
        tokio::fs::write(base.join("b.txt"), "").await.unwrap();
        tokio::fs::write(base.join("c.rs"), "").await.unwrap();

        let req = ListDirRequest {
            path: base.to_path_buf(),
            show_hidden: true,
            sort_by: SortBy::Extension,
            sort_dir: SortDir::Ascending,
            filter: None,
        };
        let res = provider().list_dir(req).await.unwrap();
        let names: Vec<_> = res.entries.iter().map(|e| e.name.clone()).collect();
        assert_eq!(names, vec!["a.rs", "c.rs", "b.txt"]);
    }

    #[tokio::test]
    async fn test_symlink_detection() {
        let tmp = TempDir::new().unwrap();
        let base = tmp.path();
        tokio::fs::write(base.join("real.txt"), "data").await.unwrap();
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(base.join("real.txt"), base.join("link.txt")).unwrap();
        }
        #[cfg(not(unix))]
        {
            // skip symlink test on non-unix
            return;
        }

        let prov = LocalFsProvider::new(false); // don't follow
        let req = ListDirRequest {
            path: base.to_path_buf(),
            show_hidden: true,
            sort_by: SortBy::Name,
            sort_dir: SortDir::Ascending,
            filter: None,
        };
        let res = prov.list_dir(req).await.unwrap();
        let link = res.entries.iter().find(|e| e.name == "link.txt").unwrap();
        assert!(link.is_symlink);
        assert_eq!(link.symlink_target, Some(base.join("real.txt")));
    }

    #[tokio::test]
    async fn test_permission_error_not_panic() {
        // We can't easily trigger a real permission error in a temp dir,
        // but we can verify that a non-existent path returns a typed error.
        let req = ListDirRequest {
            path: PathBuf::from("/nonexistent_path_42"),
            show_hidden: true,
            sort_by: SortBy::Name,
            sort_dir: SortDir::Ascending,
            filter: None,
        };
        let err = provider().list_dir(req).await.unwrap_err();
        let msg = format!("{}", err);
        assert!(msg.contains("not found") || msg.contains("NotFound"), "{}", msg);
    }

    #[tokio::test]
    async fn test_metadata() {
        let tmp = TempDir::new().unwrap();
        let base = tmp.path();
        tokio::fs::write(base.join("file"), "content").await.unwrap();

        let entry = provider()
            .metadata(base.join("file"))
            .await
            .unwrap();
        assert_eq!(entry.name, "file");
        assert_eq!(entry.size, 7);
        assert!(entry.modified.is_some());
        assert!(entry.permissions.is_some());
    }

    #[tokio::test]
    async fn test_exists() {
        let tmp = TempDir::new().unwrap();
        let base = tmp.path();
        tokio::fs::write(base.join("file"), "").await.unwrap();
        assert!(provider().exists(base.join("file")).await.unwrap());
        assert!(!provider().exists(base.join("nope")).await.unwrap());
    }
}
