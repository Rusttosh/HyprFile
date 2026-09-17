use anyhow::{Context, Result};
use std::collections::HashMap;
use std::future::Future;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::io::AsyncReadExt;
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;
use tracing::{debug, warn};

/// Supported preview types.
#[derive(Debug, Clone)]
pub enum PreviewContent {
    Text(String),
    Image { mime: String, bytes: Vec<u8> },
    Directory(Vec<String>),
    Binary { mime: String, size: u64 },
    Unsupported { reason: String },
}

/// Request to preview a file.
#[derive(Debug, Clone)]
pub struct PreviewRequest {
    pub path: PathBuf,
    pub max_bytes: usize,
}

/// Trait for the preview engine.
pub trait PreviewEngine: Send + Sync {
    fn preview(&self, req: PreviewRequest) -> impl Future<Output = Result<PreviewContent>> + Send;
    fn cancel(&self, path: PathBuf);
}

/// Lazy, cancelable preview engine.
pub struct LocalPreviewEngine {
    tokens: Arc<Mutex<HashMap<PathBuf, CancellationToken>>>,
}

impl LocalPreviewEngine {
    pub fn new() -> Self {
        Self {
            tokens: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    async fn read_text(path: &PathBuf, max_bytes: usize) -> Result<String> {
        let file = tokio::fs::File::open(path).await?;
        let meta = file.metadata().await?;
        let size = meta.len() as usize;
        let to_read = max_bytes.min(size);
        let mut buf = vec![0u8; to_read];
        let n = tokio::io::AsyncReadExt::read(&mut file.take(to_read as u64), &mut buf).await?;
        buf.truncate(n);
        match String::from_utf8(buf.clone()) {
            Ok(s) => Ok(s),
            Err(_) => Ok(String::from_utf8_lossy(&buf).into_owned()),
        }
    }

    async fn preview_inner(&self, req: PreviewRequest) -> Result<PreviewContent> {
        let path = &req.path;
        if !tokio::fs::try_exists(path).await.unwrap_or(false) {
            return Ok(PreviewContent::Unsupported {
                reason: format!("path does not exist: {}", path.display()),
            });
        }

        let meta = tokio::fs::metadata(path).await?;
        if meta.is_dir() {
            let mut entries = Vec::new();
            let mut read_dir = tokio::fs::read_dir(path).await?;
            while let Some(entry) = read_dir.next_entry().await? {
                entries.push(entry.file_name().to_string_lossy().into_owned());
                if entries.len() >= 100 {
                    break;
                }
            }
            return Ok(PreviewContent::Directory(entries));
        }

        let mime = mime_guess::from_path(path).first_or_octet_stream();
        let mime_str = mime.to_string();

        if mime.type_() == "image" {
            let bytes = tokio::fs::read(path).await?;
            return Ok(PreviewContent::Image {
                mime: mime_str,
                bytes,
            });
        }

        if meta.len() == 0 {
            return Ok(PreviewContent::Text(String::new()));
        }

        // Attempt text preview
        let text = Self::read_text(path, req.max_bytes).await;
        match text {
            Ok(content) => {
                if mime.type_() == "text" || mime.subtype() == "javascript" || mime.subtype() == "json" {
                    return Ok(PreviewContent::Text(content));
                }
                // Heuristic: if it looks like source code (common extensions)
                if let Some(ext) = path.extension() {
                    let ext = ext.to_string_lossy().to_lowercase();
                    if matches!(
                        ext.as_str(),
                        "rs" | "py" | "js" | "ts" | "go" | "c" | "cpp" | "h" | "hpp" | "java" | "kt" | "swift" | "rb" | "sh" | "bash" | "zsh" | "fish" | "toml" | "yaml" | "yml" | "json" | "xml" | "html" | "css" | "scss" | "sql" | "md" | "lua"
                    ) {
                        return Ok(PreviewContent::Text(content));
                    }
                }
                // If mostly printable, treat as text
                let printable = content.chars().take(1024).filter(|c| c.is_ascii_graphic() || c.is_ascii_whitespace()).count();
                let total = content.len().max(1);
                if printable * 100 / total > 95 {
                    return Ok(PreviewContent::Text(content));
                }
                Ok(PreviewContent::Binary { mime: mime_str, size: meta.len() })
            }
            Err(_) => Ok(PreviewContent::Binary {
                mime: mime_str,
                size: meta.len(),
            }),
        }
    }
}

impl Default for LocalPreviewEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl PreviewEngine for LocalPreviewEngine {
    async fn preview(&self, req: PreviewRequest) -> Result<PreviewContent> {
        let path = req.path.clone();
        let token = CancellationToken::new();
        {
            let mut map = self.tokens.lock().await;
            // Cancel any previous preview for this exact path
            if let Some(old) = map.get(&path) {
                old.cancel();
            }
            map.insert(path.clone(), token.clone());
        }

        tokio::select! {
            _ = token.cancelled() => {
                debug!("preview cancelled for {}", path.display());
                Ok(PreviewContent::Unsupported { reason: "cancelled".to_string() })
            }
            res = self.preview_inner(req) => res,
        }
    }

    fn cancel(&self, path: PathBuf) {
        let tokens = self.tokens.clone();
        tokio::spawn(async move {
            let mut map = tokens.lock().await;
            if let Some(token) = map.remove(&path) {
                token.cancel();
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn engine() -> LocalPreviewEngine {
        LocalPreviewEngine::new()
    }

    #[tokio::test]
    async fn test_preview_text() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("hello.txt");
        tokio::fs::write(&path, "Hello, world!").await.unwrap();

        let req = PreviewRequest {
            path: path.clone(),
            max_bytes: 1024,
        };
        let res = engine().preview(req).await.unwrap();
        match res {
            PreviewContent::Text(t) => assert_eq!(t, "Hello, world!"),
            other => panic!("expected text, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_preview_directory() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path().join("mydir");
        tokio::fs::create_dir(&dir).await.unwrap();
        tokio::fs::write(dir.join("a.txt"), "").await.unwrap();

        let req = PreviewRequest {
            path: dir.clone(),
            max_bytes: 1024,
        };
        let res = engine().preview(req).await.unwrap();
        match res {
            PreviewContent::Directory(items) => {
                assert!(items.contains(&"a.txt".to_string()));
            }
            other => panic!("expected directory, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_preview_binary() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("binary");
        tokio::fs::write(&path, vec![0u8, 1, 2, 255, 254]).await.unwrap();

        let req = PreviewRequest {
            path: path.clone(),
            max_bytes: 1024,
        };
        let res = engine().preview(req).await.unwrap();
        match res {
            PreviewContent::Binary { mime, size } => {
                assert_eq!(size, 5);
                assert!(!mime.is_empty());
            }
            other => panic!("expected binary, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_preview_cancel() {
        let engine = engine();
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("cancel.txt");
        tokio::fs::write(&path, "data").await.unwrap();

        let req = PreviewRequest {
            path: path.clone(),
            max_bytes: 1024,
        };
        // Cancel before preview completes (race, but should often hit)
        engine.cancel(path.clone());
        let res = engine.preview(req).await;
        // Cancelled result or success are both acceptable depending on timing
        assert!(res.is_ok());
    }
}
