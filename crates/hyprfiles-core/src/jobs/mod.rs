use anyhow::{Context, Result};
use std::collections::HashMap;
use std::future::Future;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc, RwLock};
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

/// Unique identifier for a job.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct JobId(pub uuid::Uuid);

impl JobId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for JobId {
    fn default() -> Self {
        Self::new()
    }
}

/// Types of file operations supported by the job queue.
#[derive(Debug, Clone)]
pub enum JobKind {
    Copy { src: PathBuf, dst: PathBuf },
    Move { src: PathBuf, dst: PathBuf },
    Rename { path: PathBuf, new_name: String },
    Trash { paths: Vec<PathBuf> },
    DeletePermanent { paths: Vec<PathBuf> },
    CreateDir { path: PathBuf },
    CreateFile { path: PathBuf },
    Compress { paths: Vec<PathBuf>, archive: PathBuf },
    Extract { archive: PathBuf, dst: PathBuf },
}

/// Current state of a job.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JobState {
    Pending,
    Running,
    Paused,
    Completed,
    Cancelled,
    Failed,
}

/// Conflict resolution strategy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConflictResolution {
    Overwrite,
    AutoRename,
    Skip,
    Cancel,
}

/// Progress update for a running job.
#[derive(Debug, Clone)]
pub struct JobProgress {
    pub job_id: JobId,
    pub processed_bytes: u64,
    pub total_bytes: u64,
    pub processed_items: u64,
    pub total_items: u64,
}

/// Internal job record.
#[derive(Debug, Clone)]
struct JobRecord {
    id: JobId,
    kind: JobKind,
    state: JobState,
    progress: JobProgress,
    error: Option<String>,
    cancel_token: CancellationToken,
}

/// Trait for the job queue engine.
pub trait JobQueue: Send + Sync {
    fn enqueue(&self, kind: JobKind) -> impl Future<Output = Result<JobId>> + Send;
    fn cancel(&self, id: JobId) -> impl Future<Output = Result<()>> + Send;
    fn state(&self, id: JobId) -> impl Future<Output = Result<JobState>> + Send;
    fn progress(
        &self,
        id: JobId,
    ) -> impl Future<Output = Result<Option<JobProgress>>> + Send;
    fn set_conflict_resolution(
        &self,
        resolution: ConflictResolution,
    ) -> impl Future<Output = ()> + Send;
    fn subscribe_progress(&self) -> impl Future<Output = broadcast::Receiver<JobProgress>> + Send;
    fn subscribe_state(&self) -> impl Future<Output = broadcast::Receiver<(JobId, JobState)>> + Send;
}

/// Async job queue backed by a Tokio task.
pub struct TokioJobQueue {
    inner: Arc<RwLock<JobQueueInner>>,
    cmd_tx: mpsc::UnboundedSender<QueueCommand>,
    progress_tx: broadcast::Sender<JobProgress>,
    state_tx: broadcast::Sender<(JobId, JobState)>,
}

#[derive(Debug)]
enum QueueCommand {
    Enqueue(JobKind, tokio::sync::oneshot::Sender<JobId>),
    Cancel(JobId),
    SetConflictResolution(ConflictResolution),
}

struct JobQueueInner {
    jobs: HashMap<JobId, JobRecord>,
    conflict_resolution: ConflictResolution,
}

impl TokioJobQueue {
    /// Create and start the job queue.
    pub fn new() -> Self {
        let (progress_tx, _) = broadcast::channel(128);
        let (state_tx, _) = broadcast::channel(128);
        let (cmd_tx, mut cmd_rx) = mpsc::unbounded_channel::<QueueCommand>();

        let inner = Arc::new(RwLock::new(JobQueueInner {
            jobs: HashMap::new(),
            conflict_resolution: ConflictResolution::AutoRename,
        }));

        let inner_clone = inner.clone();
        let progress_tx_clone = progress_tx.clone();
        let state_tx_clone = state_tx.clone();

        tokio::spawn(async move {
            while let Some(cmd) = cmd_rx.recv().await {
                match cmd {
                    QueueCommand::Enqueue(kind, reply) => {
                        let id = JobId::new();
                        let cancel_token = CancellationToken::new();
                        let record = JobRecord {
                            id,
                            kind: kind.clone(),
                            state: JobState::Pending,
                            progress: JobProgress {
                                job_id: id,
                                processed_bytes: 0,
                                total_bytes: 0,
                                processed_items: 0,
                                total_items: 0,
                            },
                            error: None,
                            cancel_token: cancel_token.clone(),
                        };
                        {
                            let mut guard = inner_clone.write().await;
                            guard.jobs.insert(id, record);
                        }
                        let _ = reply.send(id);

                        // Spawn job worker
                        let inner_worker = inner_clone.clone();
                        let ptx = progress_tx_clone.clone();
                        let stx = state_tx_clone.clone();
                        let resolution = {
                            let guard = inner_clone.read().await;
                            guard.conflict_resolution
                        };
                        tokio::spawn(async move {
                            Self::run_job(id, kind, cancel_token, inner_worker, ptx, stx, resolution).await;
                        });
                    }
                    QueueCommand::Cancel(id) => {
                        let guard = inner_clone.read().await;
                        if let Some(rec) = guard.jobs.get(&id) {
                            rec.cancel_token.cancel();
                        }
                    }
                    QueueCommand::SetConflictResolution(res) => {
                        let mut guard = inner_clone.write().await;
                        guard.conflict_resolution = res;
                    }
                }
            }
        });

        Self {
            inner,
            cmd_tx,
            progress_tx,
            state_tx,
        }
    }

    async fn run_job(
        id: JobId,
        kind: JobKind,
        token: CancellationToken,
        inner: Arc<RwLock<JobQueueInner>>,
        ptx: broadcast::Sender<JobProgress>,
        stx: broadcast::Sender<(JobId, JobState)>,
        conflict: ConflictResolution,
    ) {
        Self::set_state(id, JobState::Running, &inner, &stx).await;

        let result = tokio::select! {
            _ = token.cancelled() => {
                info!("job {} cancelled", id.0);
                Self::set_state(id, JobState::Cancelled, &inner, &stx).await;
                return;
            }
            res = Self::execute_job(id, kind.clone(), token.clone(), ptx.clone(), conflict) => res,
        };

        match result {
            Ok(()) => {
                Self::set_state(id, JobState::Completed, &inner, &stx).await;
            }
            Err(e) => {
                let msg = format!("{}", e);
                warn!("job {} failed: {}", id.0, msg);
                {
                    let mut guard = inner.write().await;
                    if let Some(rec) = guard.jobs.get_mut(&id) {
                        rec.error = Some(msg);
                    }
                }
                Self::set_state(id, JobState::Failed, &inner, &stx).await;
            }
        }
    }

    async fn set_state(
        id: JobId,
        state: JobState,
        inner: &Arc<RwLock<JobQueueInner>>,
        stx: &broadcast::Sender<(JobId, JobState)>,
    ) {
        {
            let mut guard = inner.write().await;
            if let Some(rec) = guard.jobs.get_mut(&id) {
                rec.state = state;
            }
        }
        let _ = stx.send((id, state));
    }

    async fn execute_job(
        id: JobId,
        kind: JobKind,
        token: CancellationToken,
        ptx: broadcast::Sender<JobProgress>,
        conflict: ConflictResolution,
    ) -> Result<()> {
        match kind {
            JobKind::Copy { src, dst } => {
                Self::copy_recursive(id, &src, &dst, token, ptx, conflict).await
            }
            JobKind::Move { src, dst } => {
                Self::copy_recursive(id, &src, &dst, token.clone(), ptx.clone(), conflict).await?;
                tokio::fs::remove_file(&src).await.ok();
                Ok(())
            }
            JobKind::Rename { path, new_name } => {
                let parent = path.parent().context("no parent directory")?;
                let new_path = parent.join(new_name);
                tokio::fs::rename(&path, new_path).await?;
                Ok(())
            }
            JobKind::Trash { paths } => {
                for p in paths {
                    token.cancelled().await;
                    trash::delete(&p)?;
                }
                Ok(())
            }
            JobKind::DeletePermanent { paths } => {
                for p in paths {
                    if token.is_cancelled() {
                        return Ok(());
                    }
                    if p.is_dir() {
                        tokio::fs::remove_dir_all(&p).await?;
                    } else {
                        tokio::fs::remove_file(&p).await?;
                    }
                }
                Ok(())
            }
            JobKind::CreateDir { path } => {
                tokio::fs::create_dir_all(&path).await?;
                Ok(())
            }
            JobKind::CreateFile { path } => {
                tokio::fs::write(&path, b"").await?;
                Ok(())
            }
            JobKind::Compress { .. } | JobKind::Extract { .. } => {
                // Not implemented in core yet
                Err(anyhow::anyhow!("compress/extract not yet implemented"))
            }
        }
    }

    async fn copy_recursive(
        id: JobId,
        src: &PathBuf,
        dst: &PathBuf,
        token: CancellationToken,
        ptx: broadcast::Sender<JobProgress>,
        conflict: ConflictResolution,
    ) -> Result<()> {
        if token.is_cancelled() {
            return Ok(());
        }

        let meta = tokio::fs::symlink_metadata(src).await?;
        if meta.is_dir() {
            tokio::fs::create_dir_all(dst).await?;
            let mut read_dir = tokio::fs::read_dir(src).await?;
            while let Some(entry) = read_dir.next_entry().await? {
                if token.is_cancelled() {
                    return Ok(());
                }
                let name = entry.file_name();
                let new_src = src.join(&name);
                let new_dst = dst.join(&name);
                Box::pin(Self::copy_recursive(id, &new_src, &new_dst, token.clone(), ptx.clone(), conflict)).await?;
            }
        } else {
            let final_dst = if dst.exists() {
                match conflict {
                    ConflictResolution::Overwrite => dst.clone(),
                    ConflictResolution::AutoRename => {
                        let mut counter = 1;
                        let mut candidate = dst.clone();
                        while candidate.exists() {
                            let stem = dst.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
                            let ext = dst.extension().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
                            let new_name = if ext.is_empty() {
                                format!("{}_{}", stem, counter)
                            } else {
                                format!("{}_{}.{}", stem, counter, ext)
                            };
                            candidate = dst.with_file_name(new_name);
                            counter += 1;
                        }
                        candidate
                    }
                    ConflictResolution::Skip => return Ok(()),
                    ConflictResolution::Cancel => {
                        return Err(anyhow::anyhow!("cancelled by conflict resolution"));
                    }
                }
            } else {
                dst.clone()
            };

            tokio::fs::copy(src, &final_dst).await?;
            let progress = JobProgress {
                job_id: id,
                processed_bytes: meta.len(),
                total_bytes: meta.len(),
                processed_items: 1,
                total_items: 1,
            };
            let _ = ptx.send(progress);
        }
        Ok(())
    }
}

impl Default for TokioJobQueue {
    fn default() -> Self {
        Self::new()
    }
}

impl JobQueue for TokioJobQueue {
    async fn enqueue(&self, kind: JobKind) -> Result<JobId> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        self.cmd_tx
            .send(QueueCommand::Enqueue(kind, tx))
            .map_err(|_| anyhow::anyhow!("job queue closed"))?;
        let id = rx.await.map_err(|_| anyhow::anyhow!("enqueue failed"))?;
        Ok(id)
    }

    async fn cancel(&self, id: JobId) -> Result<()> {
        self.cmd_tx
            .send(QueueCommand::Cancel(id))
            .map_err(|_| anyhow::anyhow!("job queue closed"))?;
        Ok(())
    }

    async fn state(&self, id: JobId) -> Result<JobState> {
        let guard = self.inner.read().await;
        let rec = guard
            .jobs
            .get(&id)
            .context("job not found")?;
        Ok(rec.state)
    }

    async fn progress(&self, id: JobId) -> Result<Option<JobProgress>> {
        let guard = self.inner.read().await;
        let rec = guard
            .jobs
            .get(&id)
            .context("job not found")?;
        Ok(Some(rec.progress.clone()))
    }

    async fn set_conflict_resolution(&self, resolution: ConflictResolution) {
        let _ = self.cmd_tx.send(QueueCommand::SetConflictResolution(resolution));
    }

    async fn subscribe_progress(&self) -> broadcast::Receiver<JobProgress> {
        self.progress_tx.subscribe()
    }

    async fn subscribe_state(&self) -> broadcast::Receiver<(JobId, JobState)> {
        self.state_tx.subscribe()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn queue() -> TokioJobQueue {
        TokioJobQueue::new()
    }

    #[tokio::test]
    async fn test_enqueue_copy() {
        let q = queue();
        let tmp = TempDir::new().unwrap();
        let src = tmp.path().join("src");
        tokio::fs::write(&src, "hello").await.unwrap();
        let dst = tmp.path().join("dst");

        let id = q
            .enqueue(JobKind::Copy {
                src,
                dst: dst.clone(),
            })
            .await
            .unwrap();

        // Wait a bit for completion
        for _ in 0..50 {
            tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
            if q.state(id).await.unwrap() == JobState::Completed {
                break;
            }
        }

        assert_eq!(q.state(id).await.unwrap(), JobState::Completed);
        assert!(tokio::fs::try_exists(&dst).await.unwrap());
    }

    #[tokio::test]
    async fn test_cancel_job() {
        let q = queue();
        let tmp = TempDir::new().unwrap();
        let src = tmp.path().join("src");
        tokio::fs::write(&src, "hello").await.unwrap();
        let dst = tmp.path().join("dst");

        let id = q
            .enqueue(JobKind::Copy {
                src,
                dst: dst.clone(),
            })
            .await
            .unwrap();

        q.cancel(id).await.unwrap();

        for _ in 0..20 {
            tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
            let s = q.state(id).await.unwrap();
            if s == JobState::Cancelled || s == JobState::Completed {
                break;
            }
        }

        // Cancel may race with completion; either is acceptable
        let s = q.state(id).await.unwrap();
        assert!(s == JobState::Cancelled || s == JobState::Completed);
    }

    #[tokio::test]
    async fn test_create_dir() {
        let q = queue();
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("new_dir");

        let id = q.enqueue(JobKind::CreateDir { path: path.clone() }).await.unwrap();
        for _ in 0..50 {
            tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
            if q.state(id).await.unwrap() == JobState::Completed {
                break;
            }
        }
        assert_eq!(q.state(id).await.unwrap(), JobState::Completed);
        assert!(tokio::fs::try_exists(&path).await.unwrap());
    }

    #[tokio::test]
    async fn test_rename() {
        let q = queue();
        let tmp = TempDir::new().unwrap();
        let old = tmp.path().join("old.txt");
        tokio::fs::write(&old, "").await.unwrap();
        let new = tmp.path().join("new.txt");

        let id = q
            .enqueue(JobKind::Rename {
                path: old.clone(),
                new_name: "new.txt".to_string(),
            })
            .await
            .unwrap();

        for _ in 0..50 {
            tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
            if q.state(id).await.unwrap() == JobState::Completed {
                break;
            }
        }

        assert_eq!(q.state(id).await.unwrap(), JobState::Completed);
        assert!(!tokio::fs::try_exists(&old).await.unwrap());
        assert!(tokio::fs::try_exists(&new).await.unwrap());
    }

    #[tokio::test]
    async fn test_conflict_auto_rename() {
        let q = queue();
        q.set_conflict_resolution(ConflictResolution::AutoRename).await;

        let tmp = TempDir::new().unwrap();
        let src = tmp.path().join("file.txt");
        tokio::fs::write(&src, "src").await.unwrap();
        let dst = tmp.path().join("file.txt");
        tokio::fs::write(&dst, "existing").await.unwrap();

        let id = q
            .enqueue(JobKind::Copy {
                src,
                dst: dst.clone(),
            })
            .await
            .unwrap();

        for _ in 0..50 {
            tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
            if q.state(id).await.unwrap() == JobState::Completed {
                break;
            }
        }

        assert_eq!(q.state(id).await.unwrap(), JobState::Completed);
        assert!(tokio::fs::try_exists(tmp.path().join("file_1.txt")).await.unwrap());
    }
}
