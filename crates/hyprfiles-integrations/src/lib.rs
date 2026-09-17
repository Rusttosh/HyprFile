pub mod desktop_entries;
pub mod hyprland;
pub mod portals;
pub mod terminal;
pub mod trash;
pub mod xdg;

use anyhow::Result;

/// Trait for environment detection.
pub trait EnvironmentDetector: Send + Sync {
    async fn detect(&self) -> Result<DesktopEnvironment>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DesktopEnvironment {
    Hyprland,
    Sway,
    Gnome,
    Kde,
    OtherWayland,
    X11,
    Unknown,
}
