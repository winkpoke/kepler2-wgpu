use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;
use uuid::Uuid;

/// Lifecycle states for a segmentation task.
#[derive(Debug, Clone, PartialEq)]
pub enum TaskStatus {
    /// Task was created and is waiting to be submitted / picked up.
    Pending,
    /// Task is currently being executed by the AI service.
    Running,
    /// Task completed and produced a segmentation result.
    Completed,
    /// Task failed; the `error` field on `AiTask` carries the message.
    Failed,
    /// Task was cancelled by the user.
    Cancelled,
}

impl TaskStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            TaskStatus::Pending => "pending",
            TaskStatus::Running => "running",
            TaskStatus::Completed => "completed",
            TaskStatus::Failed => "failed",
            TaskStatus::Cancelled => "cancelled",
        }
    }
}

/// Internal record for a single segmentation task.
#[derive(Debug, Clone)]
pub struct AiTask {
    pub task_id: String,
    pub series_id: String,
    pub model: String,
    pub status: TaskStatus,
    /// Progress percentage in `[0, 100]`. Updated while `Running`.
    pub progress: u8,
    /// Optional human readable error message when `status == Failed`.
    pub error: Option<String>,
    /// Volume id produced by the AI service (when status == Completed).
    pub volume_id: Option<String>,
    /// Raw `.raw` bytes of the segmentation mask (when status == Completed).
    pub mask: Option<Vec<u8>>,
    /// Label table (label id -> label name) extracted from the AI response.
    pub labels: HashMap<String, String>,
    pub created_at: Instant,
    pub updated_at: Instant,
}

impl AiTask {
    fn new(series_id: String, model: String) -> Self {
        let now = Instant::now();
        Self {
            task_id: Uuid::new_v4().to_string(),
            series_id,
            model,
            status: TaskStatus::Pending,
            progress: 0,
            error: None,
            volume_id: None,
            mask: None,
            labels: HashMap::new(),
            created_at: now,
            updated_at: now,
        }
    }
}

/// Thread-safe registry of AI segmentation tasks.
///
/// `TaskManager` is shared via `Arc<RwLock<...>>` so handlers running in
/// different Tokio tasks can read / update task state without blocking
/// unrelated work.
#[derive(Debug, Default, Clone)]
pub struct TaskManager {
    inner: Arc<RwLock<HashMap<String, AiTask>>>,
}

impl TaskManager {
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a fresh task record and return its `task_id`.
    pub async fn create(&self, series_id: String, model: String) -> String {
        let task = AiTask::new(series_id, model);
        let id = task.task_id.clone();
        self.inner.write().await.insert(id.clone(), task);
        id
    }

    /// Register a task whose `task_id` was assigned by the remote AI service
    /// (i.e. the python backend). This is used after `AiService::segment`
    /// returns a new task id; the local manager mirrors that id so that
    /// `GET /api/segment/progress/:id` matches the one broadcast to clients.
    pub async fn register(&self, task_id: String, series_id: String, model: String) {
        let now = Instant::now();
        let task = AiTask {
            task_id: task_id.clone(),
            series_id,
            model,
            status: TaskStatus::Pending,
            progress: 0,
            error: None,
            volume_id: None,
            mask: None,
            labels: HashMap::new(),
            created_at: now,
            updated_at: now,
        };
        self.inner.write().await.insert(task_id, task);
    }

    /// Get a snapshot of a task by id.
    pub async fn get(&self, task_id: &str) -> Option<AiTask> {
        self.inner.read().await.get(task_id).cloned()
    }

    /// List snapshots of all known tasks.
    pub async fn list(&self) -> Vec<AiTask> {
        self.inner.read().await.values().cloned().collect()
    }

    /// Transition a task to `Running`. No-op if the task does not exist.
    pub async fn mark_running(&self, task_id: &str) {
        let mut g = self.inner.write().await;
        if let Some(t) = g.get_mut(task_id) {
            t.status = TaskStatus::Running;
            t.updated_at = Instant::now();
        }
    }

    /// Update the progress percentage (0..=100) of a running task.
    /// Values are clamped to the valid range. No-op if the task is missing.
    pub async fn update_progress(&self, task_id: &str, value: u8) {
        let clamped = value.min(100);
        let mut g = self.inner.write().await;
        if let Some(t) = g.get_mut(task_id) {
            t.progress = clamped;
            t.updated_at = Instant::now();
        }
    }

    /// Mark a task as completed and attach the produced mask and label table.
    pub async fn mark_completed(
        &self,
        task_id: &str,
        volume_id: Option<String>,
        mask: Option<Vec<u8>>,
        labels: HashMap<String, String>,
    ) {
        let mut g = self.inner.write().await;
        if let Some(t) = g.get_mut(task_id) {
            t.status = TaskStatus::Completed;
            t.progress = 100;
            t.volume_id = volume_id;
            t.mask = mask;
            t.labels = labels;
            t.error = None;
            t.updated_at = Instant::now();
        }
    }

    /// Mark a task as failed with an error message.
    pub async fn mark_failed(&self, task_id: &str, message: String) {
        let mut g = self.inner.write().await;
        if let Some(t) = g.get_mut(task_id) {
            t.status = TaskStatus::Failed;
            t.error = Some(message);
            t.updated_at = Instant::now();
        }
    }

    /// Mark a task as cancelled. No-op if the task is already in a terminal
    /// state (`Completed` / `Failed` / `Cancelled`).
    pub async fn mark_cancelled(&self, task_id: &str) {
        let mut g = self.inner.write().await;
        if let Some(t) = g.get_mut(task_id) {
            match t.status {
                TaskStatus::Completed
                | TaskStatus::Failed
                | TaskStatus::Cancelled => {}
                _ => {
                    t.status = TaskStatus::Cancelled;
                    t.updated_at = Instant::now();
                }
            }
        }
    }

    /// Remove a task and any of its in-memory mask data.
    pub async fn remove(&self, task_id: &str) -> Option<AiTask> {
        self.inner.write().await.remove(task_id)
    }
}
