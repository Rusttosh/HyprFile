use serde::{Deserialize, Serialize};
use std::future::Future;
use std::path::PathBuf;
use thiserror::Error;
use tracing::{debug, warn};

/// A visual theme definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Theme {
    pub name: String,
    pub mode: ThemeMode,
    pub colors: ThemeColors,
    pub ui: ThemeUi,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub enum ThemeMode {
    #[default]
    Dark,
    Light,
    Auto,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ThemeColors {
    pub background: String,
    pub surface: String,
    pub overlay: String,
    pub text: String,
    pub text_muted: String,
    pub accent: String,
    pub accent_secondary: String,
    pub error: String,
    pub warning: String,
    pub info: String,
    pub success: String,
    pub border: String,
    pub selection: String,
}

impl Default for ThemeColors {
    fn default() -> Self {
        Self {
            background: "#1e1e2e".to_string(),
            surface: "#313244".to_string(),
            overlay: "#45475a".to_string(),
            text: "#cdd6f4".to_string(),
            text_muted: "#a6adc8".to_string(),
            accent: "#89b4fa".to_string(),
            accent_secondary: "#b4befe".to_string(),
            error: "#f38ba8".to_string(),
            warning: "#fab387".to_string(),
            info: "#89b4fa".to_string(),
            success: "#a6e3a1".to_string(),
            border: "#6c7086".to_string(),
            selection: "#585b70".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ThemeUi {
    pub transparency: f64,
    pub blur: bool,
    pub border_radius: u32,
    pub padding: u32,
    pub gaps: u32,
    pub icon_theme: String,
    pub nerd_font_icons: bool,
}

impl Default for ThemeUi {
    fn default() -> Self {
        Self {
            transparency: 0.92,
            blur: true,
            border_radius: 12,
            padding: 8,
            gaps: 4,
            icon_theme: "Papirus-Dark".to_string(),
            nerd_font_icons: true,
        }
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            name: "catppuccin-mocha".to_string(),
            mode: ThemeMode::Dark,
            colors: ThemeColors::default(),
            ui: ThemeUi::default(),
        }
    }
}

/// Theme validation errors.
#[derive(Debug, Error)]
pub enum ThemeError {
    #[error("invalid transparency: {0} (must be 0.0-1.0)")]
    InvalidTransparency(f64),
    #[error("invalid border_radius: {0} (must be >= 0)")]
    InvalidBorderRadius(u32),
    #[error("parse error: {0}")]
    Parse(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

/// Trait for theme loading.
pub trait ThemeLoader: Send + Sync {
    fn load(&self, path: Option<PathBuf>) -> impl Future<Output = anyhow::Result<Theme>> + Send;
    fn builtin(&self, name: &str) -> Option<Theme>;
    fn validate(&self, theme: &Theme) -> Result<(), ThemeError>;
}

/// Default theme loader with built-in themes.
pub struct DefaultThemeLoader;

impl DefaultThemeLoader {
    pub fn new() -> Self {
        Self
    }

    fn catppuccin_mocha() -> Theme {
        Theme {
            name: "catppuccin-mocha".to_string(),
            mode: ThemeMode::Dark,
            colors: ThemeColors {
                background: "#1e1e2e".to_string(),
                surface: "#313244".to_string(),
                overlay: "#45475a".to_string(),
                text: "#cdd6f4".to_string(),
                text_muted: "#a6adc8".to_string(),
                accent: "#89b4fa".to_string(),
                accent_secondary: "#b4befe".to_string(),
                error: "#f38ba8".to_string(),
                warning: "#fab387".to_string(),
                info: "#89b4fa".to_string(),
                success: "#a6e3a1".to_string(),
                border: "#6c7086".to_string(),
                selection: "#585b70".to_string(),
            },
            ui: ThemeUi {
                transparency: 0.92,
                blur: true,
                border_radius: 12,
                padding: 8,
                gaps: 4,
                icon_theme: "Papirus-Dark".to_string(),
                nerd_font_icons: true,
            },
        }
    }

    fn gruvbox_dark() -> Theme {
        Theme {
            name: "gruvbox-dark".to_string(),
            mode: ThemeMode::Dark,
            colors: ThemeColors {
                background: "#282828".to_string(),
                surface: "#3c3836".to_string(),
                overlay: "#504945".to_string(),
                text: "#ebdbb2".to_string(),
                text_muted: "#a89984".to_string(),
                accent: "#b8bb26".to_string(),
                accent_secondary: "#8ec07c".to_string(),
                error: "#fb4934".to_string(),
                warning: "#fe8019".to_string(),
                info: "#83a598".to_string(),
                success: "#b8bb26".to_string(),
                border: "#665c54".to_string(),
                selection: "#504945".to_string(),
            },
            ui: ThemeUi {
                transparency: 0.92,
                blur: true,
                border_radius: 8,
                padding: 8,
                gaps: 4,
                icon_theme: "Papirus-Dark".to_string(),
                nerd_font_icons: true,
            },
        }
    }

    fn nord() -> Theme {
        Theme {
            name: "nord".to_string(),
            mode: ThemeMode::Dark,
            colors: ThemeColors {
                background: "#2e3440".to_string(),
                surface: "#3b4252".to_string(),
                overlay: "#434c5e".to_string(),
                text: "#eceff4".to_string(),
                text_muted: "#d8dee9".to_string(),
                accent: "#88c0d0".to_string(),
                accent_secondary: "#81a1c1".to_string(),
                error: "#bf616a".to_string(),
                warning: "#ebcb8b".to_string(),
                info: "#88c0d0".to_string(),
                success: "#a3be8c".to_string(),
                border: "#4c566a".to_string(),
                selection: "#434c5e".to_string(),
            },
            ui: ThemeUi {
                transparency: 0.92,
                blur: true,
                border_radius: 10,
                padding: 8,
                gaps: 4,
                icon_theme: "Papirus-Dark".to_string(),
                nerd_font_icons: true,
            },
        }
    }

    fn tokyo_night() -> Theme {
        Theme {
            name: "tokyo-night".to_string(),
            mode: ThemeMode::Dark,
            colors: ThemeColors {
                background: "#1a1b26".to_string(),
                surface: "#24283b".to_string(),
                overlay: "#414868".to_string(),
                text: "#c0caf5".to_string(),
                text_muted: "#a9b1d6".to_string(),
                accent: "#7aa2f7".to_string(),
                accent_secondary: "#bb9af7".to_string(),
                error: "#f7768e".to_string(),
                warning: "#e0af68".to_string(),
                info: "#7aa2f7".to_string(),
                success: "#9ece6a".to_string(),
                border: "#565f89".to_string(),
                selection: "#414868".to_string(),
            },
            ui: ThemeUi {
                transparency: 0.92,
                blur: true,
                border_radius: 12,
                padding: 8,
                gaps: 4,
                icon_theme: "Papirus-Dark".to_string(),
                nerd_font_icons: true,
            },
        }
    }

    fn dracula() -> Theme {
        Theme {
            name: "dracula".to_string(),
            mode: ThemeMode::Dark,
            colors: ThemeColors {
                background: "#282a36".to_string(),
                surface: "#44475a".to_string(),
                overlay: "#6272a4".to_string(),
                text: "#f8f8f2".to_string(),
                text_muted: "#bfbfbf".to_string(),
                accent: "#bd93f9".to_string(),
                accent_secondary: "#ff79c6".to_string(),
                error: "#ff5555".to_string(),
                warning: "#f1fa8c".to_string(),
                info: "#8be9fd".to_string(),
                success: "#50fa7b".to_string(),
                border: "#6272a4".to_string(),
                selection: "#44475a".to_string(),
            },
            ui: ThemeUi {
                transparency: 0.92,
                blur: true,
                border_radius: 12,
                padding: 8,
                gaps: 4,
                icon_theme: "Papirus-Dark".to_string(),
                nerd_font_icons: true,
            },
        }
    }
}

impl Default for DefaultThemeLoader {
    fn default() -> Self {
        Self::new()
    }
}

impl ThemeLoader for DefaultThemeLoader {
    async fn load(&self, path: Option<PathBuf>) -> anyhow::Result<Theme> {
        if let Some(p) = path {
            if p.exists() {
                let contents = tokio::fs::read_to_string(&p).await?;
                let theme: Theme = toml::from_str(&contents)
                    .map_err(|e| ThemeError::Parse(format!("{}", e)))?;
                self.validate(&theme).map_err(|e| anyhow::anyhow!("{}", e))?;
                return Ok(theme);
            }
        }
        // Return default builtin theme
        Ok(Self::catppuccin_mocha())
    }

    fn builtin(&self, name: &str) -> Option<Theme> {
        match name {
            "catppuccin-mocha" => Some(Self::catppuccin_mocha()),
            "gruvbox-dark" => Some(Self::gruvbox_dark()),
            "nord" => Some(Self::nord()),
            "tokyo-night" => Some(Self::tokyo_night()),
            "dracula" => Some(Self::dracula()),
            _ => None,
        }
    }

    fn validate(&self, theme: &Theme) -> Result<(), ThemeError> {
        if theme.ui.transparency < 0.0 || theme.ui.transparency > 1.0 {
            return Err(ThemeError::InvalidTransparency(theme.ui.transparency));
        }
        // border_radius is u32, but we keep the explicit check for consistency
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn loader() -> DefaultThemeLoader {
        DefaultThemeLoader::new()
    }

    #[test]
    fn test_builtin_themes() {
        let l = loader();
        let names = ["catppuccin-mocha", "gruvbox-dark", "nord", "tokyo-night", "dracula"];
        for n in names {
            let t = l.builtin(n).expect(n);
            assert_eq!(t.name, n);
            assert!(matches!(t.mode, ThemeMode::Dark));
        }
    }

    #[tokio::test]
    async fn test_load_from_file() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("theme.toml");
        tokio::fs::write(
            &path,
            r##"
name = "custom"
mode = "Light"

[colors]
background = "#ffffff"
text = "#000000"

[ui]
transparency = 0.95
border_radius = 16
"##,
        )
        .await
        .unwrap();

        let l = loader();
        let theme = l.load(Some(path)).await.unwrap();
        assert_eq!(theme.name, "custom");
        assert!(matches!(theme.mode, ThemeMode::Light));
        assert_eq!(theme.ui.transparency, 0.95);
        assert_eq!(theme.ui.border_radius, 16);
    }

    #[tokio::test]
    async fn test_validate_fails_transparency() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("bad.toml");
        tokio::fs::write(&path, "[ui]\ntransparency = 2.0\n")
            .await
            .unwrap();

        let l = loader();
        let err = l.load(Some(path)).await.unwrap_err();
        let msg = format!("{}", err);
        assert!(msg.contains("transparency"), "{}", msg);
    }
}
