//! HTTP and WebSocket request handlers for the Kepler Axum server.
//!
//! Every item here is either a route handler (bound in [`crate::server::routes`]),
//! a shared request/response type, or a helper used by the WebSocket loop.
//! The file is organised top-to-bottom by feature area:
//!
//! 1. Health check
//! 2. GPU smoke tests (native only)
//! 3. Volume upload & DICOM export
//! 4. Needle parameters (position / orientation / offset)
//! 5. AI segmentation endpoints
//! 6. WebSocket connection and command dispatch
//! 7. HV serial device control

use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};
use std::sync::Arc;
use std::time::Instant;

use axum::{
    extract::{
        ws::{Message, WebSocket},
        ConnectInfo, Multipart, Path, Query, State, WebSocketUpgrade,
    },
    http::StatusCode,
    response::IntoResponse,
    Json,
    body::Bytes,
};
use glam::{Mat3, Mat4, Vec3, Vec4};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

use crate::device::hv::HvDevice;
use crate::server::ai_handler;
use crate::server::ai_model::{CancelRequest, SegmentRequest, SegmentResponse};
use crate::data::dicom::{build_ct_dicom, FsSink, generate_uid, Patient, StudySet};
use crate::server::state::{ServerState, StoredVolume};
use crate::server::ws::{WsMessage, WsClientCommand};

/// Broadcast a [`WsMessage`] to every connected WebSocket client.
///
/// Errors are intentionally swallowed: the broadcast channel may legitimately
/// have zero subscribers (e.g. no browser is connected yet).
pub fn broadcast_message(state: &ServerState, msg: WsMessage) {
    let _ = state.ws_tx.send(msg);
}

// ============================================================================
// 1. Health check
// ============================================================================

/// Response body for `GET /api/health`.
#[derive(Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub loaded_volumes: usize,
    pub uptime: String,
}

/// `GET /api/health` — liveness probe reporting loaded volume count and uptime.
pub async fn health_check(State(state): State<ServerState>) -> Json<HealthResponse> {
    let count = state.volume_count().await; // 从 ServerState 获取已加载体积数量
    Json(HealthResponse {
        status: "ok".to_string(),
        loaded_volumes: count,
        uptime: state.start_time.clone(),
    })
}

// ============================================================================
// 2. GPU smoke tests (native only)
// ============================================================================

/// Request body for [`gpu_test`].
#[derive(Deserialize)]
pub struct GpuTestRequest {
    pub data: Vec<f32>,
}

/// Response body for [`gpu_test`].
#[derive(Serialize)]
pub struct GpuTestResponse {
    pub data: Vec<f32>,
}

/// `POST /api/gpu/test`
///
/// Native-only smoke test for the `wgpu` 30 compute backend. Multiplies every
/// input value by `2.0` on the GPU and returns the result. Returns `503` if no
/// usable GPU / `wgpu` 30 backend is available on this host.
pub async fn gpu_test(
    State(state): State<ServerState>,
    Json(req): Json<GpuTestRequest>,
) -> Result<Json<GpuTestResponse>, (StatusCode, String)> {
    let ctx = state
        .gpu
        .context()
        .await
        .map_err(|e| (StatusCode::SERVICE_UNAVAILABLE, format!("GPU unavailable: {e}")))?;
    let result = crate::gpu::run_double(&*ctx, &req.data)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("GPU compute failed: {e}")))?;
    Ok(Json(GpuTestResponse { data: result }))
}

/// Response body for [`gpu_volume_test`].
#[derive(Serialize)]
pub struct GpuVolumeTestResponse {
    pub voxel_count: usize,
    pub sample: Vec<i16>,
}

/// `POST /api/gpu/volume/test`
///
/// Native-only smoke test that pushes an already-loaded `CTVolume`'s voxel data
/// to the `wgpu` 30 compute backend **without cloning the voxel buffer** — the
/// slice is borrowed straight out of `ServerState::ct_volume`. The bytes run
/// through a pass-through compute shader and a small prefix is read back to prove
/// the upload/compute/readback path. Returns `404` if no volume is loaded.
pub async fn gpu_volume_test(
    State(state): State<ServerState>,
) -> Result<Json<GpuVolumeTestResponse>, (StatusCode, String)> {
    let ctx = state
        .gpu
        .context()
        .await
        .map_err(|e| (StatusCode::SERVICE_UNAVAILABLE, format!("GPU unavailable: {e}")))?;

    // Borrow the volume in place and upload synchronously. The `MutexGuard` is held
    // only for the sync `prepare_volume_upload` call — never across an `.await` — so
    // the handler future stays `Send`. The voxels are never cloned (zero-copy view).
    let handle = {
        let guard = state.ct_volume.lock();
        let Some(ct) = guard.as_ref() else {
            return Err((StatusCode::NOT_FOUND, "no CTVolume loaded".to_string()));
        };
        crate::gpu::prepare_volume_upload(&*ctx, ct.voxel_data()).map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("GPU volume upload failed: {e}"),
            )
        })?
    };

    let result = crate::gpu::run_volume_compute(&*ctx, handle).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("GPU volume compute failed: {e}"),
        )
    })?;

    Ok(Json(GpuVolumeTestResponse {
        voxel_count: result.voxel_count,
        sample: result.sample,
    }))
}

// ============================================================================
// 3. Volume upload & DICOM export
// ============================================================================

/// Query parameters for `POST /api/volumes/upload`.
#[derive(Deserialize)]
pub struct UploadParams {
    #[serde(default = "default_slope")]
    pub slope: f32,
    #[serde(default = "default_intercept")]
    pub intercept: f32,
    pub patient_name: Option<String>,
    pub patient_id: Option<String>,
    pub sex: Option<String>,
    pub birthdate: Option<String>,
    pub study_id: Option<String>,
    pub description: Option<String>,
    pub kv: Option<f64>,
    pub m_as: Option<f64>,
}

fn default_slope() -> f32 { 1.0 }
fn default_intercept() -> f32 { 0.0 }

/// `POST /api/volumes/upload`
///
/// Accepts a `multipart/form-data` body with an `mha` field (required) and an
/// optional `data` field, persists the MHA to disk, and registers the volume in
/// `ServerState`. Returns the generated series UID as a JSON string.
pub async fn upload_volume(
    State(state): State<ServerState>,
    Query(params): Query<UploadParams>,
    mut multipart: Multipart,
) -> Result<Json<String>, (StatusCode, String)> {
    // Hold the uploaded fields as `Bytes` (refcounted, cheap to clone if ever needed).
    // We deliberately avoid `to_vec()` here so the MHA buffer is only copied once,
    // at the point where `StoredVolume` requires an owned `Vec<u8>`.
    let mut mha_bytes: Option<Bytes> = None;
    let mut data_bytes: Option<Bytes> = None;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("Multipart parse error: {e}")))?
    {
        let name = field.name().unwrap_or("").to_string();
        let bytes = field
            .bytes()
            .await
            .map_err(|e| (StatusCode::BAD_REQUEST, format!("Field read error: {e}")))?;

        if name == "mha" {
            mha_bytes = Some(bytes);
        } else if name == "data" {
            data_bytes = Some(bytes);
        }
    }

    let mha_bytes = mha_bytes
        .ok_or_else(|| (StatusCode::BAD_REQUEST, "Missing 'mha' multipart field".to_string()))?;
    let data_bytes = data_bytes.unwrap_or_default();

    log::info!(
        "upload_volume called with mha={} bytes, data={} bytes",
        mha_bytes.len(),
        data_bytes.len()
    );

    let id = generate_uid();
    let now = chrono::Local::now();

    // Persist the MHA to disk by reference (no copy). `Bytes` derefs to `&[u8]`,
    // which `tokio::fs::write` accepts via `AsRef<[u8]>`.
    let mha_disk_path = {
        let path = state.series_path(&id);
        if let Err(e) = tokio::fs::write(&path, &mha_bytes).await {
            log::error!("Failed to persist MHA for volume {} to {:?}: {}", id, path, e);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to persist MHA to {}: {}", path.display(), e),
            ));
        }
        log::info!("Persisted MHA for volume {} ({} bytes)", id, mha_bytes.len());
        path
    };

    // Move the refcounted `Bytes` straight into `StoredVolume` — no copy is
    // made here; the buffer stays shared with the multipart body allocation.
    let stored = StoredVolume {
        id: id.clone(),
        mhx_path: Some(mha_bytes),
        data_path: if data_bytes.is_empty() {
            None
        } else {
            Some(data_bytes)
        },
        mha_disk_path,
        loaded_at: now.format("%Y-%m-%dT%H:%M:%S").to_string(),
        patient: Patient {
            patient_id: params.patient_id.clone().unwrap_or_else(|| "UPLOAD".to_string()),
            name: params.patient_name.unwrap_or_else(|| "Unknown".to_string()),
            birthdate: params.birthdate.clone(),
            sex: params.sex.clone(),
        },
        study: StudySet {
            uid: id.clone(),
            study_id: params.study_id.clone().unwrap_or_else(|| "UPLOAD".to_string()),
            patient_id: params.patient_id.unwrap_or_else(|| "UPLOAD".to_string()),
            date: now.format("%Y%m%d").to_string(),
            description: params.description.clone(),
        },
        kv: params.kv.unwrap_or(120.0),
        m_as: params.m_as.unwrap_or(200.0),
        slope: params.slope,
        intercept: params.intercept,
        patient_position: "HFS".to_string(),
        modality: "CT".to_string(),
    };

    let count_before = state.volume_count().await;
    let series_uid = state.store_raw_volume(stored).await;
    let count_after = state.volume_count().await;
    broadcast_message(
        &state,
        WsMessage::ServerStatus {
            loaded_volumes: count_after,
            message: format!("Volume {id} uploaded"),
        },
    );

    log::info!("Volume {} stored ({} -> {} volumes)", id, count_before, count_after);
    Ok(Json(series_uid))
}

/// `GET /api/dicom/build/:id`
///
/// Loads the stored volume `id` and writes it out as a DICOM series on disk.
pub async fn build_ct_dicom_axum(
    State(state): State<ServerState>,
    Path(id): Path<String>,
) -> Result<Json<StoredVolume>, (StatusCode, String)> {
    let stored = state
        .get_volume(&id)
        .await
        .ok_or_else(|| (StatusCode::NOT_FOUND, format!("Volume {id} not found")))?;
    log::info!("Volume {} found: id={}, mhx_path_size={}, data_path_size={}",
        id,
        stored.id,
        stored.mhx_path.as_ref().map(|p| p.len()).unwrap_or(0),
        stored.data_path.as_ref().map(|p| p.len()).unwrap_or(0)
    );

    let mhx_path = stored.mhx_path.as_deref().ok_or_else(|| (StatusCode::BAD_REQUEST, "MHX path is missing".to_string()))?;
    let data_path = stored.data_path.as_deref().filter(|d| !d.is_empty());

    let out_dir = std::path::PathBuf::from("C:\\user\\dicoms");
    let temp_dir = out_dir.join(format!("CT_{}", stored.id));
    std::fs::create_dir_all(&temp_dir).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to create temp dir: {e}"),
        )
    })?;

    let mut sink = FsSink {
        out_dir: temp_dir.clone(),
    };

    build_ct_dicom(
        mhx_path,
        data_path,
        &stored.patient,
        &stored.study,
        stored.kv,
        stored.m_as,
        stored.slope,
        stored.intercept,
        stored.patient_position.clone(),
        stored.modality.clone(),
        &mut sink,
    )
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to build DICOM: {e}"),
        )
    })?;

    log::info!("DICOM exported to {:?}", temp_dir);
    Ok(Json(stored))
}

// ============================================================================
// 4. Needle parameters (position / orientation / offset)
// ============================================================================

/// Position sub-object of [`UploadNeedleJson`].
#[derive(Deserialize)]
pub struct NeedlePosition {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    #[serde(default)]
    pub unit: Option<String>,
}

/// Orientation sub-object of [`UploadNeedleJson`] (rotation angles, degrees).
#[derive(Deserialize)]
pub struct NeedleOrientation {
    pub a: f32,
    pub b: f32,
    pub c: f32,
    #[serde(default)]
    pub unit: Option<String>,
}

/// JSON body for `POST /api/upload_needle_params`.
#[derive(Deserialize)]
pub struct UploadNeedleJson {
    #[serde(default)]
    pub frame_sequence: Option<u32>,
    pub position: NeedlePosition,
    pub orientation: NeedleOrientation,
    #[serde(default)]
    pub coordinate_frame: Option<String>,
}

/// `POST /api/upload_needle_params`
///
/// Receives the needle entry point (`position`) and direction angles
/// (`orientation`) from a remote device, transforms them through the current
/// patient-to-model matrix (PTM) and offset, then broadcasts a
/// [`WsMessage::NeedleSet`] so every connected renderer can draw the needle.
pub async fn upload_needle_params(
    State(state): State<ServerState>,
    ConnectInfo(peer): ConnectInfo<std::net::SocketAddr>,
    parts: axum::http::request::Parts,
    body: Option<Json<UploadNeedleJson>>,
) -> StatusCode {
    let started = Instant::now();

    let Some(Json(b)) = body else {
        return StatusCode::BAD_REQUEST;
    };
    let pos = (b.position.x, b.position.y, b.position.z);
    let dir = (b.orientation.a, b.orientation.b, b.orientation.c);
    let id = b.frame_sequence.unwrap_or(0);

    // Stream-health diagnostics: measure the arrival gap between consecutive
    // needle packets and warn when the sender is consistently late.
    static LAST_ARRIVAL_MS: AtomicI64 = AtomicI64::new(0);
    let now_ms = chrono::Local::now().timestamp_millis();
    let prev_ms = LAST_ARRIVAL_MS.swap(now_ms, Ordering::Relaxed);
    let gap_ms = now_ms - prev_ms;
    log::debug!("Needle JSON from {peer}: pos: {pos:?}, dir: {dir:?}, gap={gap_ms}ms");
    if prev_ms != 0 && gap_ms > 500 {
        log::warn!(
            "Needle stream gap {gap_ms}ms from {peer} — request arrived late (send-side or network)"
        );
    }

    // Keep-alive diagnostic: the server (hyper) keeps HTTP/1.1 connections open by
    // default; if the needle device opens a new TCP conn per POST it is client-side.
    let conn_hdr = parts
        .headers
        .get(axum::http::header::CONNECTION)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_ascii_lowercase();
    let reusable = conn_hdr.contains("keep-alive")
        || (parts.version >= axum::http::Version::HTTP_11 && !conn_hdr.contains("close"));
    static KA_WARNED: AtomicBool = AtomicBool::new(false);
    if !reusable {
        if !KA_WARNED.swap(true, Ordering::Relaxed) {
            log::warn!(
                "{peer} sends non-reusable HTTP ({:?}, Connection: {:?}) — \
                 client forces a new TCP connection per POST",
                parts.version,
                conn_hdr
            );
        }
        log::debug!("{peer} non-reusable HTTP ({:?}, Connection: {:?})", parts.version, conn_hdr);
    }

    // Model-space needle geometry: a fixed-length segment starting at the entry
    // point and pointing along the rotated -Z axis.
    let len = 400.0;
    let rot_y = Mat3::from_rotation_y(dir.1.to_radians());
    let rot_x = Mat3::from_rotation_x(dir.0.to_radians());
    let dir_m = rot_y * rot_x * Vec3::new(0.0, 0.0, -1.0);
    let tip_m = Vec3::new(pos.0, pos.1, pos.2);
    let end_m = tip_m + dir_m * len;
    debug_assert!(((tip_m - end_m).length() - len).abs() < 0.001);

    let ptm = state.get_ptm();
    log::info!("upload_needle_params: using ptm_cols={:?}", ptm.to_cols_array());

    // Axis re-mapping applied before the PTM / offset transforms.
    let axis_transform = Mat4::from_cols(
        Vec4::new(-1.0, 0.0, 0.0, 0.0),
        Vec4::new(0.0, 0.0, 1.0, 0.0),
        Vec4::new(0.0, 1.0, 0.0, 0.0),
        Vec4::new(0.0, 0.0, 0.0, 1.0),
    );

    let end_t = axis_transform.transform_point3(end_m);
    let tip_t = axis_transform.transform_point3(tip_m);

    let offset = state.get_offset();
    let offset = Mat4::from_translation(Vec3::new(offset[0], offset[1], offset[2]));
    let entry_p = offset.transform_point3(ptm.transform_point3(end_t));
    let tip_p = offset.transform_point3(ptm.transform_point3(tip_t));

    // Broadcast to every connected browser so the wasm renderer can draw the needle.
    let _ = state.ws_tx.send(WsMessage::NeedleSet {
        id,
        x: entry_p.x,
        y: entry_p.y,
        z: entry_p.z,
        lx: tip_p.x,
        ly: tip_p.y,
        lz: tip_p.z,
        r: 0.2,
        g: 0.9,
        b: 0.2,
        dir: (dir_m.x, dir_m.y, dir_m.z),
        len_mm: len,
    });

    log::info!(
        "Needle params from {peer}: entry: {entry_p:?}, tip: {tip_p:?} (handler took {:.3} ms)",
        started.elapsed().as_secs_f32() * 1000.0
    );

    StatusCode::OK
}

/// JSON body for `POST /api/needle_point_offset`.
#[derive(Deserialize)]
pub struct OffsetParams {
    x: f32,
    y: f32,
    z: f32,
    ptm: [f32; 16],
}

/// `POST /api/needle_point_offset`
///
/// Updates the shared entry-point offset and patient-to-model matrix, and echoes
/// both values back so a client can confirm what the server now holds.
pub async fn needle_point_offset(
    State(state): State<ServerState>,
    Json(params): Json<OffsetParams>,
) -> Result<Json<([f32; 3], [f32; 16])>, StatusCode> {
    log::info!(
        "needle_point_offset: offset=({:.3},{:.3},{:.3}) ptm_cols={:?}",
        params.x,
        params.y,
        params.z,
        params.ptm
    );
    state.set_offset([params.x, params.y, params.z]);
    state.set_ptm(Mat4::from_cols_array(&params.ptm));
    Ok(Json((state.get_offset(), state.get_ptm().to_cols_array())))
}

// ============================================================================
// 5. AI segmentation endpoints
// ============================================================================

/// `POST /api/segment` — register a new segmentation task and return its id.
pub async fn start_segmentation(
    State(state): State<ServerState>,
    Json(req): Json<SegmentRequest>,
) -> Result<Json<SegmentResponse>, (StatusCode, String)> {
    match ai_handler::handle_segment(state.clone(), req).await {
        Ok(resp) => Ok(Json(resp)),
        Err(e) => {
            log::error!("start_segmentation failed: {e}");
            broadcast_message(
                &state,
                WsMessage::Error {
                    message: format!("start_segmentation failed: {e}"),
                },
            );
            Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
        }
    }
}

/// `GET /api/segment/progress/:id` — polling endpoint for task progress.
pub async fn segment_progress(
    State(state): State<ServerState>,
    Path(id): Path<String>,
) -> Result<Json<crate::server::ai_model::ProgressResponse>, (StatusCode, String)> {
    match ai_handler::handle_progress(state.clone(), id).await {
        Ok(resp) => Ok(Json(resp)),
        Err(e) => {
            log::error!("segment_progress failed: {e}");
            Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
        }
    }
}

/// `GET /api/segment/result/:id` — metadata for a completed segmentation task.
///
/// Returns `409 CONFLICT` while the task is still running, and `404` if the id
/// is unknown.
pub async fn segment_result_meta(
    State(state): State<ServerState>,
    Path(id): Path<String>,
) -> Result<Json<crate::server::ai_model::SegmentResult>, (StatusCode, String)> {
    let task = state
        .tasks
        .get(&id)
        .await
        .ok_or_else(|| (StatusCode::NOT_FOUND, format!("Task {id} not found")))?;

    if !matches!(task.status, crate::server::ai_task::TaskStatus::Completed) {
        return Err((
            StatusCode::CONFLICT,
            format!("Task {id} is not completed ({:?})", task.status),
        ));
    }

    let volume = task
        .volume_id
        .clone()
        .ok_or_else(|| (StatusCode::INTERNAL_SERVER_ERROR, "Task has no volume id".into()))?;

    Ok(Json(crate::server::ai_model::SegmentResult {
        task_id: id,
        volume,
        labels: task.labels.clone(),
        // dimensions are not tracked in AiTask; clients should rely on the original CT volume.
        dimensions: (0, 0, 0),
    }))
}

/// `GET /api/segment/result/:id/raw` — download the raw mask bytes.
pub async fn segment_result_raw(
    State(state): State<ServerState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    match ai_handler::handle_download(state.clone(), id).await {
        Ok(bytes) => Ok((
            StatusCode::OK,
            [("content-type", "application/octet-stream")],
            bytes,
        )),
        Err(e) => {
            log::error!("segment_result_raw failed: {e}");
            Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
        }
    }
}

// ============================================================================
// 6. WebSocket connection & command dispatch
// ============================================================================

/// Minimal client message used only for the `ping` keep-alive handshake.
#[derive(Deserialize)]
pub struct ClientMessage {
    #[serde(default)]
    pub kind: String,
}

/// `GET /ws` — upgrade the connection to a WebSocket.
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<ServerState>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

/// Serialize a [`WsMessage`] to JSON and push it to the client as a text frame.
/// Centralises the (de)serialization so every handler emits the same wire format.
pub async fn send_json(socket: &mut WebSocket, msg: &WsMessage) {
    match serde_json::to_string(msg) {
        Ok(json) => {
            if let Err(e) = socket.send(Message::Text(json)).await {
                log::debug!("WS send_text failed: {e}");
            }
        }
        Err(e) => log::error!("WS serialize failed: {e}"),
    }
}

/// Convenience wrapper that sends an [`WsMessage::Error`] to the client.
pub async fn send_error(socket: &mut WebSocket, message: String) {
    send_json(socket, &WsMessage::Error { message }).await;
}

/// Drive a single WebSocket connection: push an initial status message, then
/// multiplex broadcast events out and client commands in until either side closes.
async fn handle_socket(mut socket: WebSocket, state: ServerState) {
    let mut rx = state.ws_tx.subscribe();

    let count = state.volume_count().await;
    let msg = WsMessage::ServerStatus {
        loaded_volumes: count,
        message: "Connected".to_string(),
    };
    send_json(&mut socket, &msg).await;

    loop {
        tokio::select! {
            // Server -> client: broadcast events.
            ws_msg = rx.recv() => {
                match ws_msg {
                    Ok(msg) => {
                        if let Ok(json) = serde_json::to_string(&msg) {
                            if socket.send(Message::Text(json)).await.is_err() {
                                break;
                            }
                        }
                    }
                    Err(broadcast::error::RecvError::Closed) => break,
                    Err(broadcast::error::RecvError::Lagged(_)) => continue,
                }
            }
            // Client -> server: commands and control frames.
            result = socket.recv() => {
                match result {
                    Some(Ok(msg)) => match msg {
                        Message::Text(text) => {
                            dispatch_text(&mut socket, &state, &text).await;
                        }
                        Message::Ping(bytes) => {
                            if socket.send(Message::Pong(bytes)).await.is_err() {
                                break;
                            }
                        }
                        Message::Close(_) => break,
                        _ => {}
                    },
                    Some(Err(_)) => break,
                    None => break,
                }
            }
        }
    }

    log::debug!("WebSocket client disconnected");
}

/// Parse and dispatch a single inbound text frame.
///
/// Tries the typed [`WsClientCommand`] first; if that fails, falls back to the
/// lightweight [`ClientMessage`] `ping` handshake.
async fn dispatch_text(socket: &mut WebSocket, state: &ServerState, text: &str) {
    if let Ok(cmd) = serde_json::from_str::<WsClientCommand>(text) {
        match cmd {
            WsClientCommand::Segment { model, series, backend } => {
                log::info!("WS: start segment model={model} series={series} backend={backend:?}");
                let req = SegmentRequest {
                    series_id: series,
                    model,
                    backend,
                };
                if let Err(e) = ai_handler::handle_segment(state.clone(), req).await {
                    log::error!("WS segment dispatch failed: {e}");
                }
            }
            WsClientCommand::SegmentCancel { task_id } => {
                log::info!("WS: cancel segment task_id={task_id}");
                if let Err(e) = ai_handler::handle_cancel(
                    state.clone(),
                    CancelRequest { task_id: task_id.clone() },
                )
                .await
                {
                    log::error!("WS segment cancel failed: {e}");
                }
            }
            WsClientCommand::SegmentProgressQuery { task_id } => {
                if let Some(t) = state.tasks.get(&task_id).await {
                    let _ = state.ws_tx.send(WsMessage::SegmentProgress {
                        task_id,
                        value: t.progress,
                    });
                }
            }
            WsClientCommand::StartCalib => {
                // parking_lot guard lives only inside this sync block — it is released before any `.await` below.
                let res = {
                    let mut hv = state.hv.lock();
                    hv.send_lin("CAL START")
                };
                match res {
                    Ok(()) => {
                        send_json(socket, &WsMessage::HvFrame { line: "CALIB_START".into() })
                            .await
                    }
                    Err(e) => send_error(socket, format!("CALIB_ERROR: {e}")).await,
                }
            }
            WsClientCommand::SendCmd { payload } => {
                log::info!("WS: send_cmd payload={payload}");
                // STM32 固件的 PC 命令表只认"意图命令"（MODE/SET/FOOT/…），
                // 其余文本会被固件 dispatch 的兜底分支直接 NAK 掉，根本到不了高压。
                // 高压协议原文（FLK?/MA?/KV120/FLF2/…）必须包上 SEND 前缀，
                // 走固件 controller.rs 的 SEND 调试直通（原样组帧转发给高压）。
                const FW_INTENTS: [&str; 14] = [
                    "MODE", "SET", "FOOT", "CAL", "TUBE", "ILOCK", "LINK", "DET",
                    "SCAN", "AX", "RESET", "SEND", "PING", "STATUS",
                ];
                let first = payload
                    .split_whitespace()
                    .next()
                    .unwrap_or("")
                    .to_ascii_uppercase();
                let line = if FW_INTENTS.contains(&first.as_str()) {
                    payload.clone()
                } else {
                    format!("SEND {payload}")
                };
                let res = {
                    let mut hv = state.hv.lock();
                    hv.send_lin(&line)
                };
                if let Err(e) = res {
                    send_error(socket, format!("CMD_ERROR: {e}")).await;
                }
            }
        }
        return;
    }

    if let Ok(client_msg) = serde_json::from_str::<ClientMessage>(text) {
        if client_msg.kind == "ping" {
            let hb = WsMessage::Heartbeat {
                timestamp: chrono::Local::now().format("%Y-%m-%dT%H:%M:%S").to_string(),
            };
            send_json(socket, &hb).await;
        }
    }
}

// ============================================================================
// 7. HV serial device control
// ============================================================================

/// JSON body for `POST /hv/connect`.
#[derive(Deserialize)]
pub struct ConnectReq {
    pub port: String,
    pub baud: Option<u32>,
}

/// JSON body for `POST /hv/send`.
#[derive(Deserialize)]
pub struct SendReq {
    pub cmd: String,
}

/// `POST /hv/connect` — open (or re-open) the HV serial port.
///
/// `ServerState::hv` is guarded by a `parking_lot::Mutex`, so the guard is
/// acquired synchronously and never held across an `.await`.
pub async fn hv_connect(
    State(state): State<ServerState>,
    Json(req): Json<ConnectReq>,
) -> Json<serde_json::Value> {
    let baud = req.baud.unwrap_or(19200);
    let result = {
        let mut hv = state.hv.lock();
        hv.connect(&req.port, baud)
    };
    match result {
        Ok(()) => {
            log::info!("[hv] connected {} @ {}", req.port, baud);
            Json(serde_json::json!({ "ok": true, "port": req.port, "baud": baud }))
        }
        Err(e) => Json(serde_json::json!({ "ok": false, "error": e })),
    }
}

/// `POST /hv/disconnect` — release the HV serial port.
pub async fn hv_disconnect(State(state): State<ServerState>) -> Json<serde_json::Value> {
    state.hv.lock().disconnect();
    log::info!("[hv] disconnected");
    Json(serde_json::json!({ "ok": true }))
}

/// `GET /hv/status` — report connection state, port name and baud rate.
pub async fn hv_status(State(state): State<ServerState>) -> Json<serde_json::Value> {
    let hv = state.hv.lock();
    let (port, baud) = hv.info();
    Json(serde_json::json!({
        "connected": hv.is_connected(),
        "port": port,
        "baud": baud,
    }))
}

/// `POST /hv/send` — transmit a LIN frame body to the HV device.
pub async fn hv_send(
    State(state): State<ServerState>,
    Json(req): Json<SendReq>,
) -> Json<serde_json::Value> {
    let result = {
        let mut hv = state.hv.lock();
        hv.send_lin(&req.cmd)
    };
    match result {
        Ok(()) => Json(serde_json::json!({ "ok": true, "sent": req.cmd })),
        Err(e) => Json(serde_json::json!({ "ok": false, "error": e })),
    }
}

/// Spawn the HV serial RX pump 
///
/// [`HvDevice::update`] performs a *blocking* port read (10 ms timeout), so it
/// must not run on the async executor. A dedicated OS thread therefore polls
/// `update()`, drains `HvDevice::event_queue` and re-broadcasts every line as a
/// strongly-typed [`WsMessage::HvFrame`] over the *existing* `ws_tx` channel —
/// no second byte channel and no change to `handle_socket`.
///
/// Draining the queue each tick also fixes the unbounded growth of
/// `event_queue`, which previously had no consumer at all.
///
/// Taking `hv` / `ws_tx` by value (rather than a whole [`ServerState`]) keeps the
/// thread independent of `ServerState: Send` and of the async runtime.
pub fn spawn_hv_pump(hv: Arc<Mutex<HvDevice>>, ws_tx: broadcast::Sender<WsMessage>) {
    std::thread::Builder::new().name("hv-rx-pump".to_string()).spawn(move || loop {
        let (lines, connected) = {
            let mut dev = hv.lock();
            dev.update();
            (std::mem::take(&mut dev.event_queue), dev.is_connected())
        };

        for line in lines {
            log::debug!("[hv] rx/tx {line}");
            let _ = ws_tx.send(WsMessage::HvFrame { line });
        }

        if !connected {
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
    }).expect("failed to spawn hv-rx-pump thread");
}