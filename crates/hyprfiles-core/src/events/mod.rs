use std::future::Future;
use std::path::PathBuf;
use tokio::sync::broadcast;
use tracing::debug;

use crate::jobs::{JobId, JobProgress};

/// Internal event types for cross-crate communication.
#[derive(Debug, Clone)]
pub enum AppEvent {
    DirChanged { path: PathBuf },
    SelectionChanged { paths: Vec<PathBuf> },
    JobStarted { id: JobId },
    JobProgress { id: JobId, progress: JobProgress },
    JobCompleted { id: JobId },
    ConfigReloaded,
    ThemeChanged { name: String },
    RequestPreview { path: PathBuf },
    OpenWith { path: PathBuf, app: Option<String> },
    TrashItems { paths: Vec<PathBuf> },
    Quit,
}

/// Trait for an event bus.
pub trait EventBus: Send + Sync {
    fn publish(&self, event: AppEvent) -> impl Future<Output = ()> + Send;
    fn subscribe(&self) -> impl Future<Output = broadcast::Receiver<AppEvent>> + Send;
}

/// Tokio-based event bus using broadcast channels.
pub struct TokioEventBus {
    tx: broadcast::Sender<AppEvent>,
}

impl TokioEventBus {
    pub fn new() -> Self {
        let (tx, _) = broadcast::channel(256);
        Self { tx }
    }

    pub fn sender(&self) -> broadcast::Sender<AppEvent> {
        self.tx.clone()
    }
}

impl Default for TokioEventBus {
    fn default() -> Self {
        Self::new()
    }
}

impl EventBus for TokioEventBus {
    async fn publish(&self, event: AppEvent) {
        debug!("publishing event: {:?}", std::mem::discriminant(&event));
        let _ = self.tx.send(event);
    }

    async fn subscribe(&self) -> broadcast::Receiver<AppEvent> {
        self.tx.subscribe()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bus() -> TokioEventBus {
        TokioEventBus::new()
    }

    #[tokio::test]
    async fn test_publish_and_subscribe() {
        let bus = bus();
        let mut rx = bus.subscribe().await;

        bus.publish(AppEvent::Quit).await;

        let ev = rx.recv().await.unwrap();
        assert!(matches!(ev, AppEvent::Quit));
    }

    #[tokio::test]
    async fn test_multiple_subscribers() {
        let bus = bus();
        let mut rx1 = bus.subscribe().await;
        let mut rx2 = bus.subscribe().await;

        bus.publish(AppEvent::DirChanged {
            path: PathBuf::from("/tmp"),
        })
        .await;

        let ev1 = rx1.recv().await.unwrap();
        let ev2 = rx2.recv().await.unwrap();

        assert!(matches!(ev1, AppEvent::DirChanged { .. }));
        assert!(matches!(ev2, AppEvent::DirChanged { .. }));
    }
}
