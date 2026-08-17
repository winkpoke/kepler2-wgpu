use std::collections::HashMap;
use tokio::time::{sleep, Duration};

use crate::server::{
    ai_model::{
        CancelRequest, CancelResponse, ProgressResponse, SegmentRequest, SegmentResponse,
    },
    state::ServerState,
    ws::WsMessage,
};

/// Submit a new segmentation task to the AI service, register it in the
/// in-process `TaskManager` and kick off a background poller that drives
/// `segment_progress` / `segment_complete` / `segment_failed` WebSocket
/// messages until the task reaches a terminal state.
pub async fn handle_segment(
    state: ServerState,
    model: String,
    series: String,
) -> anyhow::Result<SegmentResponse> {
    let req = SegmentRequest {
        series_id: series.clone(),
        model: model.clone(),
    };
    let resp = state.ai.segment(req.clone()).await?;
    let task_id = resp.task_id.clone();

    // Register the task in the local manager so `GET /api/segment/progress/:id`
    // works even before the first poll completes. The id here is the one
    // assigned by the AI service, so HTTP and WebSocket views stay in sync.
    state.tasks.register(task_id.clone(), series.clone(), model.clone()).await;
    state.tasks.mark_running(&task_id).await;

    // Notify clients that the task has been accepted.
    let _ = state.ws_tx.send(WsMessage::SegmentStarted {
        task_id: task_id.clone(),
        series_id: series.clone(),
        model: model.clone(),
    });

    // Background poller: drives progress / completion events.
    spawn_poller(state.clone(), task_id.clone(), series.clone(), model.clone());

    Ok(resp)
}

/// Poll the AI service for the current state of a task. The caller is
/// expected to have already registered the task id in `TaskManager`.
pub async fn handle_progress(
    state: ServerState,
    task_id: String,
) -> anyhow::Result<ProgressResponse> {
    let task = state.ai.progress(&task_id).await?;
    let resp = ProgressResponse {
        task_id: task.task_id.clone(),
        value: task.progress,
        status: task.status.clone(),
        message: task.message.clone(),
    };
    // Mirror the polled state into the local task manager.
    apply_polled_task(&state, &task).await;
    Ok(resp)
}

/// Cancel a running task. Updates both the local task manager and the
/// remote AI service.
pub async fn handle_cancel(
    state: ServerState,
    req: CancelRequest,
) -> anyhow::Result<CancelResponse> {
    let task = state.ai.cancel(&req.task_id).await?;
    state.tasks.mark_cancelled(&req.task_id).await;
    let _ = state.ws_tx.send(WsMessage::SegmentCancelled {
        task_id: req.task_id.clone(),
    });
    Ok(CancelResponse {
        task_id: task.task_id,
        status: task.status,
    })
}

/// Download the raw mask bytes for a completed task. The mask is returned
/// as a `Vec<u8>` and is the same payload that the Python service encoded
/// in `mask_base64` (typically uint8 label values, row-major).
pub async fn handle_download(
    state: ServerState,
    task_id: String,
) -> anyhow::Result<Vec<u8>> {
    let bytes = state.ai.download(&task_id).await?;
    state.tasks.update_progress(&task_id, 100).await;
    Ok(bytes)
}

/// Spawn a Tokio task that polls the AI service every `POLL_INTERVAL` and
/// pushes progress / completion / failure events over the broadcast
/// channel. The poller exits as soon as the task enters a terminal state.
fn spawn_poller(state: ServerState, task_id: String, series_id: String, model: String) {
    tokio::spawn(async move {
        // Capture the original request for logging once; we never modify
        // them after the first iteration.
        let _origin = format!("series={} model={}", series_id, model);
        let mut last_value: i32 = -1;
        loop {
            let task = match state.ai.progress(&task_id).await {
                Ok(t) => t,
                Err(e) => {
                    log::warn!("AI progress poll failed for {}: {}", task_id, e);
                    state
                        .tasks
                        .mark_failed(&task_id, format!("poll failed: {}", e))
                        .await;
                    let _ = state.ws_tx.send(WsMessage::SegmentFailed {
                        task_id: task_id.clone(),
                        message: format!("poll failed: {}", e),
                    });
                    return;
                }
            };

            // Forward progress only on change to avoid WS spam.
            if task.progress as i32 != last_value {
                state.tasks.update_progress(&task_id, task.progress).await;
                let _ = state.ws_tx.send(WsMessage::SegmentProgress {
                    task_id: task_id.clone(),
                    value: task.progress,
                });
                last_value = task.progress as i32;
            }

            match task.status.as_str() {
                "completed" | "complete" | "done" => {
                    let labels = task.labels.clone().unwrap_or_default();
                    let volume = task.volume.clone().unwrap_or_else(|| {
                        // Fall back to a derived id if the AI service did not
                        // assign one explicitly.
                        format!("seg-{}", task_id)
                    });
                    let mask = state.ai.download(&task_id).await.ok();
                    state
                        .tasks
                        .mark_completed(
                            &task_id,
                            Some(volume.clone()),
                            mask,
                            labels.clone(),
                        )
                        .await;
                    let _ = state.ws_tx.send(WsMessage::SegmentComplete {
                        task_id: task_id.clone(),
                        volume,
                        labels,
                    });
                    return;
                }
                "failed" | "error" => {
                    let msg = task
                        .message
                        .clone()
                        .unwrap_or_else(|| "AI service reported failure".to_string());
                    state.tasks.mark_failed(&task_id, msg.clone()).await;
                    let _ = state.ws_tx.send(WsMessage::SegmentFailed {
                        task_id: task_id.clone(),
                        message: msg,
                    });
                    return;
                }
                "cancelled" | "canceled" => {
                    state.tasks.mark_cancelled(&task_id).await;
                    let _ = state.ws_tx.send(WsMessage::SegmentCancelled {
                        task_id: task_id.clone(),
                    });
                    return;
                }
                _ => {
                    // Still running or pending — keep polling.
                }
            }

            sleep(Duration::from_millis(POLL_INTERVAL_MS)).await;

            // Stop the poller if the local manager has been cancelled by a
            // direct HTTP cancel call.
            if let Some(t) = state.tasks.get(&task_id).await {
                if matches!(t.status, crate::server::ai_task::TaskStatus::Cancelled) {
                    return;
                }
            }
        }
    });
}

/// Apply the latest polled snapshot of an `AiServiceTask` to the local
/// `TaskManager`. Used by `handle_progress` to keep both views in sync
/// when a client polls via HTTP instead of over the WebSocket.
async fn apply_polled_task(state: &ServerState, task: &crate::server::ai_model::AiServiceTask) {
    match task.status.as_str() {
        "completed" | "complete" | "done" => {
            let labels: HashMap<String, String> = task.labels.clone().unwrap_or_default();
            let volume = task
                .volume
                .clone()
                .unwrap_or_else(|| format!("seg-{}", task.task_id));
            state
                .tasks
                .mark_completed(&task.task_id, Some(volume), None, labels)
                .await;
        }
        "failed" | "error" => {
            state
                .tasks
                .mark_failed(
                    &task.task_id,
                    task.message
                        .clone()
                        .unwrap_or_else(|| "AI service reported failure".into()),
                )
                .await;
        }
        "cancelled" | "canceled" => {
            state.tasks.mark_cancelled(&task.task_id).await;
        }
        _ => {
            state.tasks.update_progress(&task.task_id, task.progress).await;
        }
    }
}

/// How often the background poller queries the AI service.
const POLL_INTERVAL_MS: u64 = 1000;
