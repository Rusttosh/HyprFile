pub mod app;
pub mod command_palette;
pub mod dialogs;
pub mod icons;
pub mod layout;
pub mod panels;
pub mod widgets;

use hyprfiles_core::events::EventBus;

/// Trait that the UI layer must implement to communicate with the core.
pub trait UiRuntime: Send + Sync {
    fn run<E: EventBus>(
        &self,
        event_bus: std::sync::Arc<E>,
    ) -> impl std::future::Future<Output = anyhow::Result<()>> + Send;
}
