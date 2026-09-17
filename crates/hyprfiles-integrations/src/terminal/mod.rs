use anyhow::{Context, Result};
use std::path::Path;
use tracing::{debug, info, warn};

/// Known terminals in order of preference for Wayland / modern Linux.
const KNOWN_TERMINALS: &[&str] = &[
    "foot",
    "alacritty",
    "kitty",
    "wezterm",
    "gnome-terminal",
    "konsole",
    "xterm",
    "rxvt",
];

/// Detect the preferred terminal.
/// Priority:
/// 1. `HYPRFILES_TERMINAL` environment variable.
/// 2. `TERM` environment variable if it matches a known terminal.
/// 3. First known terminal found in `$PATH`.
pub fn detect_terminal() -> String {
    if let Ok(term) = std::env::var("HYPRFILES_TERMINAL") {
        if !term.is_empty() {
            debug!("Using HYPRFILES_TERMINAL={}", term);
            return term;
        }
    }
    if let Ok(term) = std::env::var("TERM") {
        let lower = term.to_lowercase();
        for known in KNOWN_TERMINALS {
            if lower.contains(known) {
                debug!("Detected terminal from TERM env: {}", term);
                return term;
            }
        }
    }
    for term in KNOWN_TERMINALS {
        if is_in_path(term) {
            debug!("Found terminal in PATH: {}", term);
            return term.to_string();
        }
    }
    warn!("No known terminal found; falling back to 'xterm'");
    "xterm".to_string()
}

/// Open a terminal in the given directory.
/// Accepts an optional explicit terminal name.
pub async fn open_terminal<P: AsRef<Path>>(path: P, terminal: Option<&str>) -> Result<()> {
    let path = path.as_ref();
    let term = terminal.map(|t| t.to_string()).unwrap_or_else(detect_terminal);
    debug!("Opening terminal {} in {:?}", term, path);

    let status = match term.as_str() {
        "foot" => tokio::process::Command::new("foot")
            .arg(format!("--working-directory={}", path.display()))
            .status()
            .await,
        "alacritty" => tokio::process::Command::new("alacritty")
            .arg("--working-directory")
            .arg(path)
            .status()
            .await,
        "kitty" => tokio::process::Command::new("kitty")
            .arg("--directory")
            .arg(path)
            .status()
            .await,
        "wezterm" => tokio::process::Command::new("wezterm")
            .arg("start")
            .arg("--cwd")
            .arg(path)
            .status()
            .await,
        "gnome-terminal" => tokio::process::Command::new("gnome-terminal")
            .arg(format!("--working-directory={}", path.display()))
            .status()
            .await,
        "konsole" => tokio::process::Command::new("konsole")
            .arg("--workdir")
            .arg(path)
            .status()
            .await,
        other => tokio::process::Command::new(other)
            .current_dir(path)
            .status()
            .await,
    };

    let status = status.with_context(|| format!("Failed to execute terminal {}", term))?;
    if status.success() {
        info!("Terminal {} opened successfully", term);
        Ok(())
    } else {
        anyhow::bail!("Terminal {} exited with status {:?}", term, status.code());
    }
}

fn is_in_path(name: &str) -> bool {
    let paths = std::env::var_os("PATH").unwrap_or_default();
    std::env::split_paths(&paths).any(|dir| dir.join(name).is_file())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_terminal_env() {
        std::env::set_var("HYPRFILES_TERMINAL", "foot");
        assert_eq!(detect_terminal(), "foot");
        std::env::remove_var("HYPRFILES_TERMINAL");
    }

    #[test]
    fn test_detect_terminal_does_not_panic_without_env() {
        let original = std::env::var_os("HYPRFILES_TERMINAL");
        std::env::remove_var("HYPRFILES_TERMINAL");
        let _ = detect_terminal();
        if let Some(v) = original {
            std::env::set_var("HYPRFILES_TERMINAL", v);
        }
    }

    #[test]
    fn test_is_in_path_basic() {
        // `sh` should almost always be present on Linux.
        assert!(is_in_path("sh"));
        // Unlikely binary
        assert!(!is_in_path("hyprfiles_unlikely_binary_12345"));
    }
}
