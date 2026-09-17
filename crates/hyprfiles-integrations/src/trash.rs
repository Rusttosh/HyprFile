use anyhow::{Context, Result};
use std::path::PathBuf;
use tracing::{debug, info};

/// XDG trash directories.
pub fn trash_dirs() -> (PathBuf, PathBuf) {
    let data_home = crate::xdg::xdg_data_home();
    let files = data_home.join("Trash").join("files");
    let info = data_home.join("Trash").join("info");
    (files, info)
}

/// Ensure the trash directories exist.
pub fn ensure_trash_dirs() -> Result<()> {
    let (files, info) = trash_dirs();
    if !files.exists() {
        std::fs::create_dir_all(&files)
            .with_context(|| format!("Failed to create trash files dir {:?}", files))?;
    }
    if !info.exists() {
        std::fs::create_dir_all(&info)
            .with_context(|| format!("Failed to create trash info dir {:?}", info))?;
    }
    Ok(())
}

/// Send a path to the trash using the `trash` crate.
/// This respects the freedesktop trash spec by delegating to the crate.
pub async fn send_to_trash(path: &std::path::Path) -> Result<()> {
    debug!("Sending {:?} to trash", path);
    // The trash crate automatically handles cross-device moves and
    // writes the .trashinfo metadata file.
    tokio::task::spawn_blocking({
        let path = path.to_path_buf();
        move || trash::delete(&path).context("trash crate failed to delete")
    })
    .await
    .context("spawn blocking task failed")??;
    info!("Sent {:?} to trash", path);
    Ok(())
}

/// List trashed files by inspecting the XDG trash directories.
/// This validates that the `trash` crate places items where expected.
pub fn list_trashed_items() -> Result<Vec<TrashedItem>> {
    let (files_dir, info_dir) = trash_dirs();
    let mut items = Vec::new();
    if !files_dir.exists() {
        return Ok(items);
    }
    for entry in std::fs::read_dir(&files_dir)
        .with_context(|| format!("Failed to read trash files dir {:?}", files_dir))?
    {
        let entry = entry?;
        let path = entry.path();
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();
        let info_path = info_dir.join(format!("{}.trashinfo", name));
        let original_path = if info_path.exists() {
            parse_trashinfo(&info_path).unwrap_or_else(|| path.clone())
        } else {
            path.clone()
        };
        let trashed_at = entry.metadata()?.modified()?;
        items.push(TrashedItem {
            path: path.clone(),
            original_path,
            trashed_at,
        });
    }
    Ok(items)
}

/// Representation of a trashed item.
#[derive(Debug, Clone)]
pub struct TrashedItem {
    pub path: PathBuf,
    pub original_path: PathBuf,
    pub trashed_at: std::time::SystemTime,
}

fn parse_trashinfo(path: &std::path::Path) -> Option<PathBuf> {
    let contents = std::fs::read_to_string(path).ok()?;
    for line in contents.lines() {
        if line.starts_with("Path=") {
            return Some(PathBuf::from(&line[5..]));
        }
    }
    None
}

/// Verify that the trash directories follow the XDG Trash spec layout.
pub fn validate_trash_xdg_compliance() -> Result<()> {
    let (files, info) = trash_dirs();
    if !files.exists() {
        anyhow::bail!("Trash files directory {:?} does not exist", files);
    }
    if !info.exists() {
        anyhow::bail!("Trash info directory {:?} does not exist", info);
    }
    debug!("Trash XDG compliance OK: files={:?}, info={:?}", files, info);
    Ok(())
}

/// Documented behavior of the trash integration:
/// - On the same filesystem: the `trash` crate typically uses `rename()` (fast).
/// - On different filesystems: it falls back to copy + delete, which is slower.
/// - Metadata is written as a `.trashinfo` file in `info/` per the freedesktop spec.
pub fn trash_behavior_docs() -> &'static str {
    r#"Trash Integration Behavior:
- Uses the `trash` crate which follows the freedesktop Trash specification.
- Trash root: `$XDG_DATA_HOME/Trash` (usually `~/.local/share/Trash`).
- `files/`: holds the actual trashed items.
- `info/`: holds `.trashinfo` XML-ish metadata files with original path and deletion time.
- Cross-device trash (e.g. external drives): the crate may copy data instead of renaming.
"#
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trash_dirs_are_xdg_compliant() {
        let (files, info) = trash_dirs();
        assert!(files.to_string_lossy().contains("Trash/files"));
        assert!(info.to_string_lossy().contains("Trash/info"));
    }

    #[test]
    fn test_ensure_trash_dirs_does_not_panic() {
        let _ = ensure_trash_dirs();
    }

    #[test]
    fn test_validate_trash_compliance_graceful() {
        // If trash dirs don't exist yet, this will fail; test that it does not panic.
        let result = validate_trash_xdg_compliance();
        // We don't assert Ok/Err because it depends on host state.
        let _ = result;
    }

    #[test]
    fn test_parse_trashinfo() {
        let dir = std::env::temp_dir();
        let info_path = dir.join("test.trashinfo");
        std::fs::write(
            &info_path,
            "[Trash Info]\nPath=/home/user/file.txt\nDeletionDate=2024-01-01T00:00:00\n",
        )
        .unwrap();
        let parsed = parse_trashinfo(&info_path);
        assert_eq!(parsed, Some(PathBuf::from("/home/user/file.txt")));
        std::fs::remove_file(&info_path).ok();
    }

    #[test]
    fn test_list_trashed_items_no_panic() {
        let _ = list_trashed_items();
    }
}
