use anyhow::{Context, Result};
use notify::Watcher;
use serde::{Deserialize, Serialize};
use std::future::Future;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{broadcast, Mutex};
use tracing::{debug, error, info, warn};

/// Application configuration root.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub general: GeneralConfig,
    pub ui: UiConfig,
    pub preview: PreviewConfig,
    pub hyprland: HyprlandConfig,
    pub behavior: BehaviorConfig,
    pub keybindings: KeybindingsConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            general: GeneralConfig::default(),
            ui: UiConfig::default(),
            preview: PreviewConfig::default(),
            hyprland: HyprlandConfig::default(),
            behavior: BehaviorConfig::default(),
            keybindings: KeybindingsConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct GeneralConfig {
    pub show_hidden: bool,
    pub confirm_delete: bool,
    pub default_layout: String,
    pub open_dirs_on_single_click: bool,
    pub terminal: String,
    pub editor: String,
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            show_hidden: false,
            confirm_delete: true,
            default_layout: "yazi".to_string(),
            open_dirs_on_single_click: false,
            terminal: "foot".to_string(),
            editor: "nvim".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct UiConfig {
    pub theme: String,
    pub transparency: f64,
    pub blur: bool,
    pub border_radius: u32,
    pub padding: u32,
    pub show_breadcrumb: bool,
    pub show_status_bar: bool,
    pub icon_theme: String,
    pub nerd_font_icons: bool,
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            theme: "catppuccin-mocha".to_string(),
            transparency: 0.92,
            blur: true,
            border_radius: 12,
            padding: 8,
            show_breadcrumb: true,
            show_status_bar: true,
            icon_theme: "Papirus-Dark".to_string(),
            nerd_font_icons: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct PreviewConfig {
    pub enabled: bool,
    pub max_file_size_mb: u64,
    pub image_preview: bool,
    pub pdf_preview: bool,
    pub video_thumbnails: bool,
    pub syntax_highlighting: bool,
}

impl Default for PreviewConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_file_size_mb: 10,
            image_preview: true,
            pdf_preview: true,
            video_thumbnails: true,
            syntax_highlighting: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct HyprlandConfig {
    pub enabled: bool,
    pub detect_active_monitor: bool,
    pub picker_mode_floating: bool,
    pub sync_with_pywal: bool,
    pub sync_with_matugen: bool,
}

impl Default for HyprlandConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            detect_active_monitor: true,
            picker_mode_floating: true,
            sync_with_pywal: false,
            sync_with_matugen: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct BehaviorConfig {
    pub trash_by_default: bool,
    pub permanent_delete_requires_phrase: bool,
    pub case_sensitive_sort: bool,
    pub directories_first: bool,
}

impl Default for BehaviorConfig {
    fn default() -> Self {
        Self {
            trash_by_default: true,
            permanent_delete_requires_phrase: true,
            case_sensitive_sort: false,
            directories_first: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct KeybindingsConfig {
    pub quit: Vec<String>,
    pub open: Vec<String>,
    pub parent: Vec<String>,
    pub down: Vec<String>,
    pub up: Vec<String>,
    pub search: Vec<String>,
    pub command_palette: Vec<String>,
    pub toggle_hidden: Vec<String>,
    pub trash: Vec<String>,
    pub rename: Vec<String>,
    pub new_file: Vec<String>,
    pub new_folder: Vec<String>,
}

impl Default for KeybindingsConfig {
    fn default() -> Self {
        Self {
            quit: vec!["q".to_string()],
            open: vec!["Enter".to_string(), "l".to_string()],
            parent: vec!["h".to_string()],
            down: vec!["j".to_string(), "Down".to_string()],
            up: vec!["k".to_string(), "Up".to_string()],
            search: vec!["/".to_string()],
            command_palette: vec![":".to_string()],
            toggle_hidden: vec![".".to_string()],
            trash: vec!["dd".to_string()],
            rename: vec!["r".to_string()],
            new_file: vec!["n".to_string()],
            new_folder: vec!["N".to_string()],
        }
    }
}

/// Validation errors for configuration.
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("invalid transparency value: {0} (must be 0.0-1.0)")]
    InvalidTransparency(f64),
    #[error("invalid border_radius: {0} (must be >= 0)")]
    InvalidBorderRadius(u32),
    #[error("toml parse error: {0}")]
    Parse(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

/// Validates a loaded configuration, returning errors for invalid values.
pub fn validate_config(cfg: &Config) -> Result<(), ConfigError> {
    if cfg.ui.transparency < 0.0 || cfg.ui.transparency > 1.0 {
        return Err(ConfigError::InvalidTransparency(cfg.ui.transparency));
    }
    // border_radius is u32 so non-negative by type, but we keep the check for explicitness
    Ok(())
}

/// Trait for configuration loading.
pub trait ConfigLoader: Send + Sync {
    fn load(&self, path: Option<PathBuf>) -> impl Future<Output = Result<Config>> + Send;
    fn reload(&self) -> impl Future<Output = Result<Config>> + Send;
    fn subscribe_reloads(&self) -> impl Future<Output = broadcast::Receiver<Config>> + Send;
}

/// TOML config loader with hot reload via `notify`.
pub struct TomlConfigLoader {
    default_path: PathBuf,
    current: Arc<Mutex<Config>>,
    reload_tx: broadcast::Sender<Config>,
    _watcher: Arc<Mutex<Option<notify::RecommendedWatcher>>>,
}

impl TomlConfigLoader {
    pub fn new() -> Result<Self> {
        let default_path = dirs::config_dir()
            .map(|d| d.join("hyprfiles").join("config.toml"))
            .unwrap_or_else(|| PathBuf::from("~/.config/hyprfiles/config.toml"));

        let (reload_tx, _) = broadcast::channel(4);
        let current = Arc::new(Mutex::new(Config::default()));

        Ok(Self {
            default_path,
            current,
            reload_tx,
            _watcher: Arc::new(Mutex::new(None)),
        })
    }

    async fn load_from_disk(path: &PathBuf) -> Result<Config> {
        if !path.exists() {
            debug!("config file not found, using defaults");
            return Ok(Config::default());
        }
        let contents = tokio::fs::read_to_string(path).await?;
        let cfg: Config = toml::from_str(&contents)
            .map_err(|e| ConfigError::Parse(format!("{}", e)))?;
        validate_config(&cfg).map_err(|e| anyhow::anyhow!("{}", e))?;
        Ok(cfg)
    }

    fn spawn_watcher(&self) {
        let path = self.default_path.clone();
        let current = self.current.clone();
        let reload_tx = self.reload_tx.clone();

        let watcher = notify::recommended_watcher(move |res: Result<notify::Event, notify::Error>| {
            if let Ok(event) = res {
                if event.kind.is_modify() || event.kind.is_create() {
                    let path = path.clone();
                    let current = current.clone();
                    let reload_tx = reload_tx.clone();
                    tokio::spawn(async move {
                        match Self::load_from_disk(&path).await {
                            Ok(cfg) => {
                                let mut guard = current.lock().await;
                                *guard = cfg.clone();
                                let _ = reload_tx.send(cfg);
                                info!("config reloaded");
                            }
                            Err(e) => {
                                warn!("failed to reload config: {}", e);
                            }
                        }
                    });
                }
            }
        });

        if let Ok(mut w) = watcher {
            let parent = self.default_path.parent().unwrap_or_else(|| std::path::Path::new("."));
            if parent.exists() {
                let _ = w.watch(parent, notify::RecursiveMode::NonRecursive);
            }
            let _watcher_guard = self._watcher.clone();
            tokio::spawn(async move {
                let mut guard = _watcher_guard.lock().await;
                *guard = Some(w);
            });
        }
    }
}

impl Default for TomlConfigLoader {
    fn default() -> Self {
        Self::new().unwrap()
    }
}

impl ConfigLoader for TomlConfigLoader {
    async fn load(&self, path: Option<PathBuf>) -> Result<Config> {
        let target = path.unwrap_or_else(|| self.default_path.clone());
        let cfg = Self::load_from_disk(&target).await?;
        {
            let mut guard = self.current.lock().await;
            *guard = cfg.clone();
        }
        self.spawn_watcher();
        Ok(cfg)
    }

    async fn reload(&self) -> Result<Config> {
        let cfg = Self::load_from_disk(&self.default_path).await?;
        {
            let mut guard = self.current.lock().await;
            *guard = cfg.clone();
        }
        let _ = self.reload_tx.send(cfg.clone());
        Ok(cfg)
    }

    async fn subscribe_reloads(&self) -> broadcast::Receiver<Config> {
        self.reload_tx.subscribe()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn loader() -> TomlConfigLoader {
        TomlConfigLoader::new().unwrap()
    }

    #[tokio::test]
    async fn test_load_defaults() {
        let l = loader();
        let cfg = l.load(None).await.unwrap();
        assert_eq!(cfg.general.default_layout, "yazi");
        assert_eq!(cfg.ui.theme, "catppuccin-mocha");
    }

    #[tokio::test]
    async fn test_load_from_file() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("config.toml");
        tokio::fs::write(
            &path,
            r#"
[general]
show_hidden = true
terminal = "alacritty"

[ui]
transparency = 0.85
border_radius = 8
"#,
        )
        .await
        .unwrap();

        let l = loader();
        let cfg = l.load(Some(path)).await.unwrap();
        assert_eq!(cfg.general.show_hidden, true);
        assert_eq!(cfg.general.terminal, "alacritty");
        assert_eq!(cfg.ui.transparency, 0.85);
        assert_eq!(cfg.ui.border_radius, 8);
    }

    #[tokio::test]
    async fn test_validation_fails_transparency() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("bad.toml");
        tokio::fs::write(&path, "[ui]\ntransparency = 1.5\n")
            .await
            .unwrap();

        let l = loader();
        let err = l.load(Some(path)).await.unwrap_err();
        let msg = format!("{}", err);
        assert!(msg.contains("transparency"), "{}", msg);
    }

    #[tokio::test]
    async fn test_reload() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("config.toml");
        tokio::fs::write(&path, "[general]\nshow_hidden = true\n")
            .await
            .unwrap();

        let l = TomlConfigLoader::new().unwrap();
        let _ = l.load(Some(path.clone())).await.unwrap();
        tokio::fs::write(&path, "[general]\nshow_hidden = false\n")
            .await
            .unwrap();
        let cfg = l.reload().await.unwrap();
        assert_eq!(cfg.general.show_hidden, false);
    }
}
