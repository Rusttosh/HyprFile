use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use tracing::{debug, trace, warn};

/// Represents a parsed `.desktop` entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DesktopEntry {
    pub id: String,
    pub name: String,
    pub exec: String,
    pub icon: Option<String>,
    pub mime_types: Vec<String>,
    pub terminal: bool,
    pub no_display: bool,
    pub hidden: bool,
    pub path: Option<PathBuf>,
}

impl Default for DesktopEntry {
    fn default() -> Self {
        Self {
            id: String::new(),
            name: String::new(),
            exec: String::new(),
            icon: None,
            mime_types: Vec::new(),
            terminal: false,
            no_display: false,
            hidden: false,
            path: None,
        }
    }
}

/// Directories where `.desktop` files are stored.
pub fn application_dirs() -> Vec<PathBuf> {
    let mut dirs = vec![
        PathBuf::from("/usr/share/applications"),
        PathBuf::from("/usr/local/share/applications"),
    ];
    if let Some(data_dirs) = std::env::var_os("XDG_DATA_DIRS") {
        for p in std::env::split_paths(&data_dirs) {
            dirs.push(p.join("applications"));
        }
    }
    dirs.push(
        std::env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .or_else(|| dirs::data_dir())
            .unwrap_or_else(|| {
                let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/"));
                home.join(".local/share")
            })
            .join("applications"),
    );
    dirs.into_iter().filter(|p| p.exists()).collect()
}

/// Parse a `.desktop` file content string into a `DesktopEntry`.
pub fn parse_desktop_entry(content: &str, id: Option<&str>) -> Result<DesktopEntry> {
    let mut entry = DesktopEntry::default();
    if let Some(id) = id {
        entry.id = id.to_string();
    }

    let mut in_desktop_entry = false;
    for raw_line in content.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line == "[Desktop Entry]" {
            in_desktop_entry = true;
            continue;
        }
        if line.starts_with('[') && line.ends_with(']') {
            in_desktop_entry = false;
            continue;
        }
        if !in_desktop_entry {
            continue;
        }
        if let Some((key, value)) = line.split_once('=') {
            let key = key.trim();
            let value = value.trim();
            match key {
                "Name" => entry.name = value.to_string(),
                "Exec" => entry.exec = value.to_string(),
                "Icon" => entry.icon = Some(value.to_string()),
                "MimeType" => {
                    entry.mime_types =
                        value.split(';').map(|s| s.trim().to_string()).collect();
                }
                "Terminal" => entry.terminal = value == "true",
                "NoDisplay" => entry.no_display = value == "true",
                "Hidden" => entry.hidden = value == "true",
                _ => {}
            }
        }
    }

    if entry.name.is_empty() && entry.id.is_empty() {
        anyhow::bail!("Desktop entry has no Name or Id");
    }
    Ok(entry)
}

/// Read all `.desktop` files from the standard application directories.
pub fn list_all_entries() -> Result<Vec<DesktopEntry>> {
    let mut entries = Vec::new();
    for dir in application_dirs() {
        trace!("Scanning applications directory: {:?}", dir);
        for item in std::fs::read_dir(&dir)
            .with_context(|| format!("Failed to read dir {:?}", dir))?
        {
            let item = item?;
            let path = item.path();
            let Some(ext) = path.extension().and_then(|s| s.to_str()) else {
                continue;
            };
            if ext != "desktop" {
                continue;
            }
            let content = match std::fs::read_to_string(&path) {
                Ok(c) => c,
                Err(e) => {
                    warn!("Failed to read {:?}: {}", path, e);
                    continue;
                }
            };
            let id = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown")
                .to_string();
            match parse_desktop_entry(&content, Some(&id)) {
                Ok(mut entry) => {
                    entry.path = Some(path.clone());
                    if entry.no_display || entry.hidden {
                        trace!("Skipping hidden/NoDisplay entry {}", entry.id);
                        continue;
                    }
                    entries.push(entry);
                }
                Err(e) => {
                    warn!("Failed to parse {:?}: {}", path, e);
                }
            }
        }
    }
    Ok(entries)
}

/// Find all applications that declare support for a given MIME type.
pub fn entries_for_mime(mime_type: &str) -> Result<Vec<DesktopEntry>> {
    let all = list_all_entries()?;
    let mime_lower = mime_type.to_lowercase();
    Ok(all
        .into_iter()
        .filter(|e| {
            e.mime_types
                .iter()
                .any(|m| m.to_lowercase() == mime_lower)
        })
        .collect())
}

/// Resolve MIME type of a file and return applicable desktop entries.
pub fn entries_for_file(path: &Path) -> Result<Vec<DesktopEntry>> {
    let mime_type = guess_mime_type(path)?;
    debug!("Resolved MIME type {} for {:?}", mime_type, path);
    entries_for_mime(&mime_type)
}

/// Guess MIME type using the `mime` crate and fallback to `xdg-mime` or file extension.
pub fn guess_mime_type(path: &Path) -> Result<String> {
    if let Some(guess) = mime_guess::from_path(path) {
        return Ok(guess.to_string());
    }
    // Fallback: try `xdg-mime query filetype`
    let output = std::process::Command::new("xdg-mime")
        .args(["query", "filetype", &path.to_string_lossy()])
        .output();
    if let Ok(output) = output {
        if output.status.success() {
            let mime = String::from_utf8_lossy(&output.stdout);
            return Ok(mime.trim().to_string());
        }
    }
    anyhow::bail!("Could not determine MIME type for {:?}", path)
}

/// Build the command line to open a file with a specific desktop entry.
/// Replaces `%f` / `%F` / `%u` / `%U` with the target path.
pub fn build_exec_command(entry: &DesktopEntry, target: &Path) -> Vec<String> {
    let mut args: Vec<String> = split_exec(&entry.exec);

    let target_str = target.to_string_lossy().to_string();
    for arg in &mut args {
        *arg = arg.replace("%f", &target_str);
        *arg = arg.replace("%F", &target_str);
        *arg = arg.replace("%u", &target_str);
        *arg = arg.replace("%U", &target_str);
        // Remove other field codes we don't handle
        for code in &["%i", "%c", "%k", "%v", "%m", "%d", "%D", "%n", "%N", "%v"] {
            *arg = arg.replace(code, "");
        }
    }
    args.retain(|s| !s.is_empty());
    args
}

fn split_exec(exec: &str) -> Vec<String> {
    let mut args = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    for ch in exec.chars() {
        match ch {
            '"' => in_quotes = !in_quotes,
            ' ' if !in_quotes => {
                if !current.is_empty() {
                    args.push(current.clone());
                    current.clear();
                }
            }
            _ => current.push(ch),
        }
    }
    if !current.is_empty() {
        args.push(current);
    }
    args
}

// Re-export a lightweight MIME guessing helper.
// The `mime_guess` crate is not in the workspace, so we can use a naive fallback.
// However, since the user wants compilable code, let's see if we can use `mime`
// crate's sniffing. The `mime` crate itself only holds constants. We can do a naive
// extension mapping here.
mod mime_guess {
    use std::path::Path;
    use std::collections::HashMap;
    use std::sync::LazyLock;

    static EXT_TO_MIME: LazyLock<HashMap<&'static str, &'static str>> = LazyLock::new(|| {
        let mut m = HashMap::new();
        m.insert("txt", "text/plain");
        m.insert("md", "text/markdown");
        m.insert("html", "text/html");
        m.insert("htm", "text/html");
        m.insert("css", "text/css");
        m.insert("js", "application/javascript");
        m.insert("json", "application/json");
        m.insert("png", "image/png");
        m.insert("jpg", "image/jpeg");
        m.insert("jpeg", "image/jpeg");
        m.insert("gif", "image/gif");
        m.insert("svg", "image/svg+xml");
        m.insert("pdf", "application/pdf");
        m.insert("zip", "application/zip");
        m.insert("tar", "application/x-tar");
        m.insert("gz", "application/gzip");
        m.insert("mp3", "audio/mpeg");
        m.insert("mp4", "video/mp4");
        m.insert("webm", "video/webm");
        m.insert("rs", "text/rust");
        m.insert("toml", "application/toml");
        m.insert("desktop", "application/x-desktop");
        m
    });

    pub fn from_path(path: &Path) -> Option<mime::Mime> {
        path.extension()
            .and_then(|e| e.to_str())
            .and_then(|ext| EXT_TO_MIME.get(ext.to_lowercase().as_str()))
            .and_then(|m| m.parse::<mime::Mime>().ok())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_parse_desktop_entry_basic() {
        let content = r#"[Desktop Entry]
Name=Firefox
Exec=/usr/bin/firefox %u
Icon=firefox
MimeType=text/html;application/xhtml+xml;
Terminal=false
NoDisplay=false
"#;
        let entry = parse_desktop_entry(content, Some("firefox.desktop")).unwrap();
        assert_eq!(entry.name, "Firefox");
        assert_eq!(entry.exec, "/usr/bin/firefox %u");
        assert_eq!(entry.icon, Some("firefox".to_string()));
        assert_eq!(entry.mime_types, vec!["text/html", "application/xhtml+xml", ""]);
        assert!(!entry.terminal);
        assert!(!entry.no_display);
    }

    #[test]
    fn test_parse_hidden_skipped() {
        let content = r#"[Desktop Entry]
Name=HiddenApp
Exec=/bin/true
Hidden=true
"#;
        let entry = parse_desktop_entry(content, Some("hidden.desktop")).unwrap();
        assert!(entry.hidden);
    }

    #[test]
    fn test_entries_for_mime_from_temp_dir() {
        let dir = tempfile::tempdir().unwrap();
        let app_dir = dir.path().join("applications");
        std::fs::create_dir_all(&app_dir).unwrap();

        let mut f = std::fs::File::create(app_dir.join("test.desktop")).unwrap();
        write!(
            f,
            r#"[Desktop Entry]
Name=TestApp
Exec=/usr/bin/test %f
MimeType=text/plain;
"#
        )
        .unwrap();

        // Override XDG_DATA_HOME temporarily
        let original = std::env::var_os("XDG_DATA_HOME");
        std::env::set_var("XDG_DATA_HOME", dir.path());
        let apps = entries_for_mime("text/plain").unwrap();
        if let Some(ref orig) = original {
            std::env::set_var("XDG_DATA_HOME", orig);
        } else {
            std::env::remove_var("XDG_DATA_HOME");
        }

        assert!(
            apps.iter().any(|a| a.name == "TestApp"),
            "Expected TestApp among entries for text/plain"
        );
    }

    #[test]
    fn test_build_exec_command() {
        let entry = DesktopEntry {
            exec: "/usr/bin/foo %f".to_string(),
            ..Default::default()
        };
        let cmd = build_exec_command(&entry, Path::new("/tmp/hello.txt"));
        assert_eq!(cmd, vec!["/usr/bin/foo", "/tmp/hello.txt"]);
    }

    #[test]
    fn test_guess_mime_type_by_extension() {
        let mime = guess_mime_type(Path::new("file.txt")).unwrap();
        assert_eq!(mime, "text/plain");
    }
}
