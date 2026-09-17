use std::path::PathBuf;
use tracing::{debug, trace, warn};

/// Detect whether `xdg-desktop-portal-hyprland` is installed.
/// Looks for the service file or the shared module.
pub fn detect_xdg_desktop_portal_hyprland() -> bool {
    // Check for the portal impl file in common libexec/lib dirs.
    let candidates: Vec<PathBuf> = vec![
        PathBuf::from("/usr/lib/xdg-desktop-portal-hyprland"),
        PathBuf::from("/usr/libexec/xdg-desktop-portal-hyprland"),
        PathBuf::from("/usr/local/lib/xdg-desktop-portal-hyprland"),
        PathBuf::from("/usr/local/libexec/xdg-desktop-portal-hyprland"),
    ];
    for candidate in &candidates {
        if candidate.exists() {
            trace!("Found xdg-desktop-portal-hyprland at {:?}", candidate);
            return true;
        }
    }
    // Also check systemd user service file presence as a heuristic.
    let service_paths = [
        PathBuf::from("/usr/share/systemd/user/xdg-desktop-portal-hyprland.service"),
        PathBuf::from("/usr/local/share/systemd/user/xdg-desktop-portal-hyprland.service"),
    ];
    for sp in &service_paths {
        if sp.exists() {
            trace!("Found systemd service file at {:?}", sp);
            return true;
        }
    }
    false
}

/// Future-proof placeholder for portal capabilities.
#[derive(Debug, Clone, Default)]
pub struct PortalCapabilities {
    pub screenshot: bool,
    pub file_chooser: bool,
    pub notification: bool,
    pub settings: bool,
}

/// Returns placeholder capabilities.
/// Real use of D-Bus portals is left for future implementation.
pub fn capabilities() -> PortalCapabilities {
    let has_hyprland_portal = detect_xdg_desktop_portal_hyprland();
    debug!(
        "xdg-desktop-portal-hyprland detected: {}",
        has_hyprland_portal
    );
    PortalCapabilities {
        screenshot: has_hyprland_portal,
        file_chooser: has_hyprland_portal,
        notification: has_hyprland_portal,
        settings: has_hyprland_portal,
    }
}

/// Open a file chooser portal (placeholder).
/// In the future this should talk to `org.freedesktop.portal.Desktop` over D-Bus.
pub async fn open_file_chooser() {
    warn!("open_file_chooser is a placeholder; D-Bus portal integration not yet implemented");
}

/// Take a screenshot via portal (placeholder).
pub async fn take_screenshot() {
    warn!("take_screenshot is a placeholder; D-Bus portal integration not yet implemented");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_portal_does_not_panic() {
        let _ = detect_xdg_desktop_portal_hyprland();
    }

    #[test]
    fn test_capabilities_default() {
        let caps = capabilities();
        // We don't assert true/false because it depends on the host system.
        // Just ensure it compiles and runs.
        let _ = caps.screenshot;
    }
}
