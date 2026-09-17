use anyhow::{Context, Result};
use std::future::Future;
use std::path::PathBuf;
use tracing::{debug, warn};

/// MIME type and associated applications.
#[derive(Debug, Clone)]
pub struct MimeInfo {
    pub mime_type: String,
    pub description: String,
    pub default_app: Option<String>,
    pub apps: Vec<String>,
}

/// Trait for MIME handling.
pub trait MimeResolver: Send + Sync {
    fn resolve(&self, path: PathBuf) -> impl Future<Output = Result<MimeInfo>> + Send;
    fn open_with_default(&self, path: PathBuf) -> impl Future<Output = Result<()>> + Send;
    fn open_with(&self, path: PathBuf, app: String) -> impl Future<Output = Result<()>> + Send;
    fn copy_path(&self, path: PathBuf) -> String;
    fn copy_name(&self, path: PathBuf) -> String;
    fn copy_uri(&self, path: PathBuf) -> String;
}

/// Desktop entry parsed from a `.desktop` file.
#[derive(Debug, Clone, Default)]
pub struct DesktopEntry {
    pub name: String,
    pub exec: String,
    pub icon: Option<String>,
}

/// Default MIME resolver using `mime_guess` and `xdg-open`.
pub struct DefaultMimeResolver;

impl DefaultMimeResolver {
    pub fn new() -> Self {
        Self
    }

    /// Parse a `.desktop` file, returning basic fields.
    pub fn parse_desktop_entry(path: &PathBuf) -> Result<DesktopEntry> {
        let content = std::fs::read_to_string(path)?;
        let mut entry = DesktopEntry::default();
        let mut in_desktop_entry = false;
        for line in content.lines() {
            if line.trim() == "[Desktop Entry]" {
                in_desktop_entry = true;
                continue;
            }
            if line.starts_with('[') {
                in_desktop_entry = false;
                continue;
            }
            if !in_desktop_entry {
                continue;
            }
            if let Some((k, v)) = line.split_once('=') {
                match k.trim() {
                    "Name" => entry.name = v.trim().to_string(),
                    "Exec" => entry.exec = v.trim().to_string(),
                    "Icon" => entry.icon = Some(v.trim().to_string()),
                    _ => {}
                }
            }
        }
        Ok(entry)
    }
}

impl Default for DefaultMimeResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl MimeResolver for DefaultMimeResolver {
    async fn resolve(&self, path: PathBuf) -> Result<MimeInfo> {
        let mime = mime_guess::from_path(&path).first_or_octet_stream();
        let mime_type = mime.to_string();

        // Try to find default app via xdg-mime query default
        let default_app = tokio::task::spawn_blocking({
            let mime_type = mime_type.clone();
            move || {
                let output = std::process::Command::new("xdg-mime")
                    .args(["query", "default", &mime_type])
                    .output()
                    .ok()?;
                if output.status.success() {
                    let s = String::from_utf8_lossy(&output.stdout).trim().to_string();
                    if !s.is_empty() {
                        return Some(s);
                    }
                }
                None
            }
        })
        .await
        .unwrap_or(None);

        Ok(MimeInfo {
            mime_type,
            description: String::new(),
            default_app,
            apps: Vec::new(),
        })
    }

    async fn open_with_default(&self, path: PathBuf) -> Result<()> {
        debug!("opening {} with default app", path.display());
        let status = tokio::process::Command::new("xdg-open")
            .arg(&path)
            .status()
            .await
            .with_context(|| format!("failed to run xdg-open for {}", path.display()))?;
        if !status.success() {
            return Err(anyhow::anyhow!("xdg-open exited with {}", status));
        }
        Ok(())
    }

    async fn open_with(&self, path: PathBuf, app: String) -> Result<()> {
        debug!("opening {} with {}", path.display(), app);
        let status = tokio::process::Command::new(&app)
            .arg(&path)
            .status()
            .await
            .with_context(|| format!("failed to run {} for {}", app, path.display()))?;
        if !status.success() {
            return Err(anyhow::anyhow!("{} exited with {}", app, status));
        }
        Ok(())
    }

    fn copy_path(&self, path: PathBuf) -> String {
        path.to_string_lossy().into_owned()
    }

    fn copy_name(&self, path: PathBuf) -> String {
        path.file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default()
    }

    fn copy_uri(&self, path: PathBuf) -> String {
        format!("file://{}", path.to_string_lossy())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn resolver() -> DefaultMimeResolver {
        DefaultMimeResolver::new()
    }

    #[tokio::test]
    async fn test_resolve_mime() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("test.txt");
        tokio::fs::write(&path, "hello").await.unwrap();

        let info = resolver().resolve(path).await.unwrap();
        assert_eq!(info.mime_type, "text/plain");
    }

    #[tokio::test]
    async fn test_resolve_image() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("image.png");
        tokio::fs::write(&path, b"\x89PNG\r\n\x1a\n").await.unwrap();

        let info = resolver().resolve(path).await.unwrap();
        assert_eq!(info.mime_type, "image/png");
    }

    #[tokio::test]
    async fn test_copy_helpers() {
        let r = resolver();
        let path = PathBuf::from("/home/user/doc.txt");
        assert_eq!(r.copy_path(path.clone()), "/home/user/doc.txt");
        assert_eq!(r.copy_name(path.clone()), "doc.txt");
        assert_eq!(r.copy_uri(path.clone()), "file:///home/user/doc.txt");
    }

    #[test]
    fn test_parse_desktop_entry() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("test.desktop");
        std::fs::write(
            &path,
            "[Desktop Entry]\nName=TestApp\nExec=/usr/bin/test\nIcon=test-icon\n",
        )
        .unwrap();

        let entry = DefaultMimeResolver::parse_desktop_entry(&path).unwrap();
        assert_eq!(entry.name, "TestApp");
        assert_eq!(entry.exec, "/usr/bin/test");
        assert_eq!(entry.icon, Some("test-icon".to_string()));
    }
}
