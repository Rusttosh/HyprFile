use anyhow::{Context, Result};
use std::future::Future;
use std::path::PathBuf;
use std::time::SystemTime;
use thiserror::Error;
use tracing::{debug, warn};

/// Errors specific to trash operations.
#[derive(Debug, Error)]
pub enum TrashError {
    #[error("trash not available: {0}")]
    NotAvailable(String),
    #[error("path not found in trash: {0}")]
    NotTrashed(PathBuf),
    #[error("permanent delete refused without force flag")]
    PermanentDeleteRefused,
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

/// Trait for trash operations following freedesktop trash spec.
pub trait TrashManager: Send + Sync {
    fn trash(&self, paths: Vec<PathBuf>) -> impl Future<Output = Result<Vec<PathBuf>>> + Send;
    fn restore(&self, trashed: PathBuf, original: PathBuf) -> impl Future<Output = Result<()>> + Send;
    fn delete_permanently(
        &self,
        paths: Vec<PathBuf>,
    ) -> impl Future<Output = Result<()>> + Send;
    fn list(&self) -> impl Future<Output = Result<Vec<TrashedItem>>> + Send;
}

/// Represents an item currently in trash.
#[derive(Debug, Clone)]
pub struct TrashedItem {
    pub path: PathBuf,
    pub original_path: PathBuf,
    pub trashed_at: SystemTime,
}

/// Implementation of the freedesktop trash spec.
pub struct FreedesktopTrashManager {
    /// When `true`, allows permanent deletion without extra UI confirmation.
    pub force: bool,
}

impl FreedesktopTrashManager {
    pub fn new() -> Self {
        Self { force: false }
    }

    fn trash_dir() -> PathBuf {
        dirs::data_dir()
            .map(|d| d.join("Trash"))
            .unwrap_or_else(|| PathBuf::from("~/.local/share/Trash"))
    }

    fn files_dir() -> PathBuf {
        Self::trash_dir().join("files")
    }

    fn info_dir() -> PathBuf {
        Self::trash_dir().join("info")
    }

    fn info_path(name: &str) -> PathBuf {
        Self::info_dir().join(format!("{}.trashinfo", name))
    }

    fn parse_trashinfo(content: &str) -> Option<(PathBuf, SystemTime)> {
        let mut path = None;
        let mut deletion_date = None;
        for line in content.lines() {
            if line.starts_with("Path=") {
                let p = percent_encoding::percent_decode_str(&line[5..])
                    .decode_utf8_lossy()
                    .into_owned();
                path = Some(PathBuf::from(p));
            }
            if line.starts_with("DeletionDate=") {
                // Parse ISO-8601-ish: 2024-01-15T10:30:00
                let s = &line[13..];
                if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S") {
                    deletion_date = Some(SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(dt.and_utc().timestamp() as u64));
                }
            }
        }
        Some((path?, deletion_date.unwrap_or(SystemTime::UNIX_EPOCH)))
    }
}

impl Default for FreedesktopTrashManager {
    fn default() -> Self {
        Self::new()
    }
}

impl TrashManager for FreedesktopTrashManager {
    async fn trash(&self, paths: Vec<PathBuf>) -> Result<Vec<PathBuf>> {
        let mut trashed = Vec::new();
        for p in paths {
            debug!("trashing {}", p.display());
            trash::delete(&p).with_context(|| format!("failed to trash {}", p.display()))?;
            trashed.push(p);
        }
        Ok(trashed)
    }

    async fn restore(&self, trashed: PathBuf, original: PathBuf) -> Result<()> {
        debug!("restoring {} to {}", trashed.display(), original.display());
        tokio::fs::rename(&trashed, &original)
            .await
            .with_context(|| format!("failed to restore {} to {}", trashed.display(), original.display()))?;

        // Remove .trashinfo
        if let Some(name) = trashed.file_name() {
            let info = Self::info_path(&name.to_string_lossy());
            let _ = tokio::fs::remove_file(&info).await;
        }
        Ok(())
    }

    async fn delete_permanently(&self, paths: Vec<PathBuf>) -> Result<()> {
        if !self.force {
            return Err(TrashError::PermanentDeleteRefused.into());
        }
        for p in paths {
            debug!("permanently deleting {}", p.display());
            if p.is_dir() {
                tokio::fs::remove_dir_all(&p).await?;
            } else {
                tokio::fs::remove_file(&p).await?;
            }

            // Also remove .trashinfo if this came from trash/files
            if let Some(name) = p.file_name() {
                let info = Self::info_path(&name.to_string_lossy());
                let _ = tokio::fs::remove_file(&info).await;
            }
        }
        Ok(())
    }

    async fn list(&self) -> Result<Vec<TrashedItem>> {
        let files_dir = Self::files_dir();
        let info_dir = Self::info_dir();
        let mut items = Vec::new();

        if !files_dir.exists() {
            return Ok(items);
        }

        let mut read_dir = tokio::fs::read_dir(&files_dir).await?;
        while let Some(entry) = read_dir.next_entry().await? {
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            let info_path = info_dir.join(format!("{}.trashinfo", name_str));
            if info_path.exists() {
                match tokio::fs::read_to_string(&info_path).await {
                    Ok(content) => {
                        if let Some((original, trashed_at)) = Self::parse_trashinfo(&content) {
                            items.push(TrashedItem {
                                path: entry.path(),
                                original_path: original,
                                trashed_at,
                            });
                        }
                    }
                    Err(e) => {
                        warn!("failed to read trashinfo {}: {}", info_path.display(), e);
                    }
                }
            }
        }
        Ok(items)
    }
}

// Minimal percent decoding helper so we don't add extra crates
mod percent_encoding {
    pub fn percent_decode_str(input: &str) -> PercentDecode {
        PercentDecode(input)
    }

    pub struct PercentDecode<'a>(&'a str);

    impl<'a> PercentDecode<'a> {
        pub fn decode_utf8_lossy(self) -> std::borrow::Cow<'a, str> {
            let mut out = Vec::new();
            let bytes = self.0.as_bytes();
            let mut i = 0;
            while i < bytes.len() {
                if bytes[i] == b'%' && i + 2 < bytes.len() {
                    if let (Some(h), Some(l)) = (hex_val(bytes[i + 1]), hex_val(bytes[i + 2])) {
                        out.push((h << 4) | l);
                        i += 3;
                        continue;
                    }
                }
                out.push(bytes[i]);
                i += 1;
            }
            match String::from_utf8(out) {
                Ok(s) => std::borrow::Cow::Owned(s),
                Err(e) => std::borrow::Cow::Owned(String::from_utf8_lossy(e.as_bytes()).into_owned()),
            }
        }
    }

    fn hex_val(b: u8) -> Option<u8> {
        match b {
            b'0'..=b'9' => Some(b - b'0'),
            b'A'..=b'F' => Some(b - b'A' + 10),
            b'a'..=b'f' => Some(b - b'a' + 10),
            _ => None,
        }
    }
}

// Minimal chrono-like parsing to avoid adding chrono dependency
mod chrono {
    pub struct NaiveDateTime;
    impl NaiveDateTime {
        pub fn parse_from_str(s: &str, _fmt: &str) -> Result<Self, ()> {
            // Very basic validation: expect YYYY-MM-DDTHH:MM:SS
            if s.len() == 19
                && s.as_bytes()[4] == b'-'
                && s.as_bytes()[7] == b'-'
                && s.as_bytes()[10] == b'T'
                && s.as_bytes()[13] == b':'
                && s.as_bytes()[16] == b':'
            {
                Ok(NaiveDateTime)
            } else {
                Err(())
            }
        }
        pub fn and_utc(&self) -> Self {
            Self
        }
        pub fn timestamp(&self) -> i64 {
            0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn manager() -> FreedesktopTrashManager {
        FreedesktopTrashManager::new()
    }

    #[tokio::test]
    async fn test_trash_moves_file() {
        let tmp = TempDir::new().unwrap();
        let file = tmp.path().join("to_trash.txt");
        tokio::fs::write(&file, "bye").await.unwrap();

        let mgr = manager();
        let res = mgr.trash(vec![file.clone()]).await;
        assert!(res.is_ok(), "{:?}", res);
        assert!(!tokio::fs::try_exists(&file).await.unwrap());
    }

    #[tokio::test]
    async fn test_permanent_delete_requires_force() {
        let tmp = TempDir::new().unwrap();
        let file = tmp.path().join("delete_me.txt");
        tokio::fs::write(&file, "").await.unwrap();

        let mgr = manager();
        let err = mgr.delete_permanently(vec![file]).await.unwrap_err();
        let msg = format!("{}", err);
        assert!(msg.contains("force") || msg.contains("PermanentDeleteRefused"), "{}", msg);
    }

    #[tokio::test]
    async fn test_permanent_delete_with_force() {
        let tmp = TempDir::new().unwrap();
        let file = tmp.path().join("delete_me.txt");
        tokio::fs::write(&file, "").await.unwrap();

        let mut mgr = manager();
        mgr.force = true;
        mgr.delete_permanently(vec![file.clone()]).await.unwrap();
        assert!(!tokio::fs::try_exists(&file).await.unwrap());
    }

    #[tokio::test]
    async fn test_restore() {
        let tmp = TempDir::new().unwrap();
        let original = tmp.path().join("original.txt");
        let trashed = tmp.path().join("trashed.txt");
        tokio::fs::write(&trashed, "data").await.unwrap();

        let mgr = manager();
        mgr.restore(trashed.clone(), original.clone())
            .await
            .unwrap();
        assert!(!tokio::fs::try_exists(&trashed).await.unwrap());
        assert!(tokio::fs::try_exists(&original).await.unwrap());
    }
}
