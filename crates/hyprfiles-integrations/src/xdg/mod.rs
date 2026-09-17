use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tracing::{debug, trace, warn};

/// Returns `XDG_CONFIG_HOME` or falls back to `~/.config`.
pub fn xdg_config_home() -> PathBuf {
    std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| dirs::config_dir())
        .unwrap_or_else(|| {
            let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/"));
            home.join(".config")
        })
}

/// Returns `XDG_CACHE_HOME` or falls back to `~/.cache`.
pub fn xdg_cache_home() -> PathBuf {
    std::env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .or_else(|| dirs::cache_dir())
        .unwrap_or_else(|| {
            let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/"));
            home.join(".cache")
        })
}

/// Returns `XDG_DATA_HOME` or falls back to `~/.local/share`.
pub fn xdg_data_home() -> PathBuf {
    std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| dirs::data_dir())
        .unwrap_or_else(|| {
            let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/"));
            home.join(".local/share")
        })
}

/// Application-specific config directory: `~/.config/hyprfiles/`.
pub fn app_config_dir() -> PathBuf {
    xdg_config_home().join("hyprfiles")
}

/// Application-specific cache directory: `~/.cache/hyprfiles/`.
pub fn app_cache_dir() -> PathBuf {
    xdg_cache_home().join("hyprfiles")
}

/// Application-specific data directory: `~/.local/share/hyprfiles/`.
pub fn app_data_dir() -> PathBuf {
    xdg_data_home().join("hyprfiles")
}

/// Returns system and user MIME database directories.
pub fn mime_dirs() -> Vec<PathBuf> {
    let mut dirs = vec![PathBuf::from("/usr/share/mime")];
    if let Some(data) = std::env::var_os("XDG_DATA_DIRS") {
        for p in std::env::split_paths(&data) {
            dirs.push(p.join("mime"));
        }
    }
    dirs.push(xdg_data_home().join("mime"));
    dirs.into_iter().filter(|p| p.exists()).collect()
}

/// Read MIME type descriptions from the system MIME database.
/// Returns a mapping of MIME type -> description.
pub fn read_mime_database() -> Result<HashMap<String, String>> {
    let mut result = HashMap::new();
    for dir in mime_dirs() {
        let globs_file = dir.join("globs2");
        if globs_file.exists() {
            trace!("Reading MIME globs2 from {:?}", globs_file);
            // globs2 format: "2:pattern:mime\n"
            let contents = std::fs::read_to_string(&globs_file)
                .with_context(|| format!("Failed to read {:?}", globs_file))?;
            for line in contents.lines() {
                let parts: Vec<&str> = line.splitn(3, ':').collect();
                if parts.len() == 3 {
                    result.entry(parts[2].to_string()).or_default();
                }
            }
        }
        let packages_dir = dir.join("packages");
        if packages_dir.is_dir() {
            for entry in std::fs::read_dir(&packages_dir)
                .with_context(|| format!("Failed to read dir {:?}", packages_dir))?
            {
                let entry = entry?;
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("xml") {
                    trace!("Skipping deep XML parse for {:?}", path);
                    // Basic extraction of mime-type and comment via simple string scan.
                    let text = std::fs::read_to_string(&path).unwrap_or_default();
                    result.extend(scan_mime_types_from_xml(&text));
                }
            }
        }
    }
    Ok(result)
}

fn scan_mime_types_from_xml(text: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    let mut current_mime: Option<String> = None;
    for line in text.lines() {
        let line = line.trim();
        if line.starts_with("<mime-type type=\"") {
            if let Some(start) = line.find("type=\"") {
                let start = start + 6;
                if let Some(end) = line[start..].find('\"') {
                    current_mime = Some(line[start..start + end].to_string());
                }
            }
        } else if line.starts_with("<comment>") && current_mime.is_some() {
            let end = line.find("</comment>").unwrap_or(line.len());
            let comment = line[9..end].trim().to_string();
            map.insert(current_mime.take().unwrap(), comment);
        }
    }
    map
}

/// Open a file or URL using `xdg-open`.
pub async fn xdg_open<P: AsRef<Path>>(path: P) -> Result<()> {
    let path = path.as_ref();
    debug!("Opening via xdg-open: {:?}", path);
    let status = tokio::process::Command::new("xdg-open")
        .arg(path)
        .status()
        .await
        .with_context(|| format!("Failed to execute xdg-open for {:?}", path))?;
    if status.success() {
        Ok(())
    } else {
        anyhow::bail!("xdg-open exited with non-zero status: {:?}", status)
    }
}

/// Return directories where icon themes are installed.
pub fn icon_theme_dirs() -> Vec<PathBuf> {
    let mut dirs = vec![
        PathBuf::from("/usr/share/icons"),
        PathBuf::from("/usr/local/share/icons"),
    ];
    dirs.push(xdg_data_home().join("icons"));
    if let Some(data_dirs) = std::env::var_os("XDG_DATA_DIRS") {
        for p in std::env::split_paths(&data_dirs) {
            dirs.push(p.join("icons"));
        }
    }
    dirs.into_iter().filter(|p| p.exists()).collect()
}

/// List available icon themes by scanning `index.theme` files.
pub fn list_icon_themes() -> Vec<String> {
    let mut themes = Vec::new();
    for dir in icon_theme_dirs() {
        if let Ok(entries) = std::fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() && path.join("index.theme").exists() {
                    if let Some(name) = path.file_name().and_then(|s| s.to_str()) {
                        themes.push(name.to_string());
                    }
                }
            }
        }
    }
    themes.sort();
    themes.dedup();
    themes
}

/// Resolve an icon name to a path within the given theme (or hicolor fallback).
/// Very basic resolution: looks for `name`.svg / `name`.png under theme subdirs.
pub fn resolve_icon(name: &str, theme: Option<&str>) -> Option<PathBuf> {
    let theme_dirs = icon_theme_dirs();
    let themes_to_check: Vec<String> = theme
        .map(|t| vec![t.to_string(), "hicolor".to_string()])
        .unwrap_or_else(|| vec!["hicolor".to_string()]);
    for theme_name in &themes_to_check {
        for dir in &theme_dirs {
            let theme_path = dir.join(theme_name);
            if !theme_path.exists() {
                continue;
            }
            // Search subdirectories like scalable/apps, 48x48/apps, etc.
            if let Ok(entries) = std::fs::read_dir(&theme_path) {
                for sub in entries.flatten() {
                    let sub_path = sub.path();
                    if !sub_path.is_dir() {
                        continue;
                    }
                    let apps_dir = sub_path.join("apps");
                    if !apps_dir.exists() {
                        continue;
                    }
                    for ext in &["svg", "png", "xpm"] {
                        let candidate = apps_dir.join(format!("{}.{}", name, ext));
                        if candidate.exists() {
                            return Some(candidate);
                        }
                    }
                }
            }
        }
    }
    warn!("Icon '{}' not found in themes {:?}", name, themes_to_check);
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_xdg_dirs_not_empty() {
        let cfg = xdg_config_home();
        assert!(!cfg.as_os_str().is_empty());
        let cache = xdg_cache_home();
        assert!(!cache.as_os_str().is_empty());
        let data = xdg_data_home();
        assert!(!data.as_os_str().is_empty());
    }

    #[test]
    fn test_app_dirs_relative_to_xdg() {
        let app_cfg = app_config_dir();
        assert!(app_cfg.file_name().unwrap() == "hyprfiles");
    }

    #[test]
    fn test_mime_dirs_exist_or_empty() {
        let dirs = mime_dirs();
        // On a normal Linux system at least /usr/share/mime should exist,
        // but in a minimal container it may not. Test only that it doesn't panic.
        for d in &dirs {
            assert!(d.exists());
        }
    }

    #[test]
    fn test_read_mime_database_does_not_panic() {
        let _ = read_mime_database();
    }

    #[test]
    fn test_icon_theme_dirs_not_empty_on_linux() {
        let dirs = icon_theme_dirs();
        // Should contain at least /usr/share/icons if it exists
        // We just assert no panic and valid paths.
        for d in &dirs {
            assert!(d.is_absolute());
        }
    }

    #[test]
    fn test_list_icon_themes_no_panic() {
        let _themes = list_icon_themes();
    }

    #[test]
    fn test_scan_mime_types_from_xml() {
        let xml = r#"<mime-type type="text/plain">
            <comment>Plain text</comment>
        </mime-type>"#;
        let map = scan_mime_types_from_xml(xml);
        assert_eq!(map.get("text/plain"), Some(&"Plain text".to_string()));
    }
}
