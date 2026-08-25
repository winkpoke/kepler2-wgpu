use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::server::handlers::{Circle, CalibrationResult};

/// WebSocket message types sent from server to client.
/// 服务器返回 / 推送给客户端的消息
#[derive(Serialize, Clone)]
#[serde(tag = "type")]
pub enum WsMessage {
    #[serde(rename = "parsing_progress")]
    ParsingProgress {
        volume_id: String,
        current_file: usize,
        total_files: usize,
    },
    #[serde(rename = "export_progress")]
    ExportProgress {
        volume_id: String,
        current_slice: usize,
        total_slices: usize,
    },
    #[serde(rename = "server_status")]
    ServerStatus {
        loaded_volumes: usize,
        message: String,
    },
    #[serde(rename = "error")]
    Error {
        message: String,
    },
    #[serde(rename = "heartbeat")]
    Heartbeat {
        timestamp: String,
    },
    // ---------- AI segmentation (Section 8 of the design doc) ----------
    /// Sent when a `POST /api/segment` request has been accepted and a new task has been registered.
    #[serde(rename = "segment_started")]
    SegmentStarted {
        task_id: String,
        series_id: String,
        model: String,
    },
    /// Periodic progress update for a running task. `value` is in `[0, 100]`.
    #[serde(rename = "segment_progress")]
    SegmentProgress {
        task_id: String,
        value: u8,
    },
    /// Task finished successfully and the mask is now available.
    #[serde(rename = "segment_complete")]
    SegmentComplete {
        task_id: String,
        /// Server-side volume id that the mask is registered under.
        volume: String,
        /// Label id -> anatomical name. Stringified keys for JSON friendliness.
        labels: HashMap<String, String>,
    },
    /// Task failed; clients should surface the message to the user.
    #[serde(rename = "segment_failed")]
    SegmentFailed {
        task_id: String,
        message: String,
    },
    /// A remote client set needle parameters via `POST /api/upload_needle_params`.
    #[serde(rename = "needle_set")]
    NeedleSet {
        id: u32,
        x: f32,
        y: f32,
        z: f32,
        lx: f32,
        ly: f32,
        lz: f32,
        r: f32,
        g: f32,
        b: f32,
        dir: (f32, f32, f32),
        len_mm: f32,
    },
    /// Task was cancelled (either by the user or by the AI service).
    #[serde(rename = "segment_cancelled")]
    SegmentCancelled {
        task_id: String,
    },
    Slice {
        slice: usize,
        width: usize,
        height: usize,
        data: Vec<u16>,
    },
    CircleAccepted {
        slice: usize,
        circle: Circle,
    },
    Progress {
        stage: String,
        progress: f32,
    },
    CalibrationResult {
        result: CalibrationResult,
    },
    Finished,
}

/// Client → server WebSocket commands. The discriminator lives in `type`.
/// 客户端发送给服务器的命令
#[derive(Deserialize)]
#[serde(tag = "type")]
pub enum WsClientCommand {
    /// Start a new segmentation task. Mirrors the body of `POST /api/segment`.
    #[serde(rename = "segment")]
    Segment {
        model: String,
        series: String,
    },
    /// Cancel an in-flight segmentation task.
    #[serde(rename = "segment_cancel")]
    SegmentCancel {
        task_id: String,
    },
    /// Query the current progress of a task (the server replies with a
    /// `SegmentProgress` message).
    #[serde(rename = "segment_progress_query")]
    SegmentProgressQuery {
        task_id: String,
    },
    LoadSlice {
        slice: usize,
        width: usize,
        height: usize,
    },
    ConfirmCircle {
        slice: usize,
        circle: Circle,
    },
    RunCalibration,
    EccCancel,
}
