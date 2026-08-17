use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Body for `POST /api/segment`.
///
/// `series_id` is the volume id uploaded via `/api/volumes/upload`; `model`
/// is the AI model name (e.g. `totalsegmentator`).
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SegmentRequest {
    pub series_id: String,
    pub model: String,
}

/// Response for `POST /api/segment`. Returned synchronously after the task
/// has been registered. Clients should subscribe to WebSocket messages
/// (e.g. `segment_started`, `segment_progress`, `segment_complete`) to
/// observe progress.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SegmentResponse {
    pub status: String,
    pub task_id: String,
}

/// Response for `GET /api/segment/progress/:id` and the periodic polling
/// the Rust server performs against the Python AI service.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ProgressResponse {
    pub task_id: String,
    /// Progress percentage in `[0, 100]`.
    pub value: u8,
    /// Lifecycle status as a string (`pending` / `running` / `completed` /
    /// `failed` / `cancelled`).
    pub status: String,
    /// Optional human-readable error message, populated when `status == "failed"`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

/// Response for `GET /api/segment/result/:id` and the WebSocket
/// `segment_complete` payload.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SegmentResult {
    pub task_id: String,
    /// Server-side volume id of the segmentation mask. Clients can use this
    /// id to request a download via `GET /api/segment/result/:id/raw`.
    pub volume: String,
    /// Label table: numeric label value (as string) -> anatomical name.
    pub labels: HashMap<String, String>,
    /// `width x height x depth` of the mask in voxels.
    pub dimensions: (u32, u32, u32),
}

/// Body for `POST /api/segment/cancel`.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CancelRequest {
    pub task_id: String,
}

/// Response for `POST /api/segment/cancel`.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CancelResponse {
    pub task_id: String,
    pub status: String,
}

/// Internal wire format used by the Python AI service. Used as a request
/// body for `POST /segment` and as the response body for `GET /task/{id}`.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AiServiceTask {
    pub task_id: String,
    pub status: String,
    #[serde(default)]
    pub progress: u8,
    #[serde(default)]
    pub message: Option<String>,
    #[serde(default)]
    pub volume: Option<String>,
    #[serde(default)]
    pub labels: Option<HashMap<String, String>>,
    #[serde(default)]
    pub mask_base64: Option<String>,
}
