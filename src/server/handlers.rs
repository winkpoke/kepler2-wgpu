use axum::{
    extract::{ConnectInfo, Multipart, Path, Query, State, WebSocketUpgrade},
    extract::ws::{WebSocket, Message},
    http::StatusCode,
    response::IntoResponse,
    Json,
    body::{Body, Bytes},
    response::Response,
};
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;
use bincode;

use crate::server::ai_handler;
use crate::server::ai_model::{CancelRequest, SegmentRequest, SegmentResponse};
use crate::data::dicom::{build_ct_dicom, FsSink, generate_uid, Patient, StudySet};
use crate::server::state::{ServerState, StoredVolume};
use crate::server::ws::{WsMessage, WsClientCommand};
use crate::rendering::view::mesh::Mesh;
use glam::{Vec3, Vec4, Mat4, Mat3};

/// Client WebSocket message
#[derive(serde::Deserialize)]
pub struct ClientMessage {
    #[serde(default)]
    pub kind: String,
}

#[derive(Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub loaded_volumes: usize,
    pub uptime: String,
}

pub async fn health_check(State(state): State<ServerState>) -> Json<HealthResponse> {
    let count = state.volume_count().await; // 从 ServerState 获取已加载体积数量
    Json(HealthResponse {
        status: "ok".to_string(),
        loaded_volumes: count,
        uptime: state.start_time.clone(),
    })
}

/// WebSocket handler for real-time events
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<ServerState>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

async fn handle_socket(mut socket: WebSocket, state: ServerState) {
    let mut rx = state.ws_tx.subscribe();
    let count = state.volume_count().await;
    let msg = WsMessage::ServerStatus {
        loaded_volumes: count,
        message: "Connected".to_string(),
    };
    if let Ok(json) = serde_json::to_string(&msg) {
        let _ = socket.send(Message::Text(json)).await;
    }

    loop {
        tokio::select! {
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
            result = socket.recv() => {
                match result {
                    Some(Ok(msg)) => match msg {
                        Message::Text(text) => {
                            if let Ok(cmd) = serde_json::from_str::<WsClientCommand>(&text) {
                                match cmd {
                                    WsClientCommand::Segment { model, series } => {
                                        log::info!("WS: start segment model={} series={}", model, series);
                                        if let Err(e) = ai_handler::handle_segment(state.clone(), model, series).await {
                                            log::error!("WS segment dispatch failed: {e}");
                                        }
                                    }
                                    WsClientCommand::SegmentCancel { task_id } => {
                                        log::info!("WS: cancel segment task_id={}", task_id);
                                        if let Err(e) = ai_handler::handle_cancel(
                                            state.clone(),
                                            CancelRequest { task_id: task_id.clone() },
                                        ).await {
                                            log::error!("WS segment cancel failed: {e}");
                                        }
                                    }
                                    WsClientCommand::SegmentProgressQuery { task_id } => {
                                        if let Some(t) = state.tasks.get(&task_id).await {
                                            let _ = state.ws_tx.send(WsMessage::SegmentProgress {
                                                task_id: task_id.clone(),
                                                value: t.progress,
                                            });
                                        }
                                    }
                                    WsClientCommand::LoadSlice { slice, width, height } => {
                                        handle_load_slice(&mut socket, &state, slice, width, height).await;
                                    }
                                    WsClientCommand::ConfirmCircle { slice, circle } => {
                                        handle_confirm_circle(&mut socket, &state, slice, circle).await;
                                    }
                                    WsClientCommand::RunCalibration => {
                                        handle_run_calibration(&mut socket, &state).await;
                                    }
                                    WsClientCommand::EccCancel => {
                                        // 后面可以实现 cancellation token
                                    }
                                }
                            } else if let Ok(client_msg) = serde_json::from_str::<ClientMessage>(&text) {
                                if client_msg.kind == "ping" {
                                    let hb = WsMessage::Heartbeat {
                                        timestamp: chrono::Local::now().format("%Y-%m-%dT%H:%M:%S").to_string(),
                                    };
                                    if let Ok(json) = serde_json::to_string(&hb) {
                                        if socket.send(Message::Text(json)).await.is_err() {
                                            break;
                                        }
                                    }
                                }
                            }
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

/// Send a WebSocket message to all connected clients
pub fn broadcast_message(state: &ServerState, msg: WsMessage) {
    let _ = state.ws_tx.send(msg);
}

/// Query parameters for volume upload
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

/// JSON body 输入：position / orientation 嵌套结构
#[derive(Deserialize)]
pub struct NeedlePosition {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    #[serde(default)]
    pub unit: Option<String>,
}

#[derive(Deserialize)]
pub struct NeedleOrientation {
    pub a: f32,
    pub b: f32,
    pub c: f32,
    #[serde(default)]
    pub unit: Option<String>,
}

#[derive(Deserialize)]
pub struct UploadNeedleJson {
    #[serde(default)]
    pub frame_sequence: Option<u32>,
    pub position: NeedlePosition,
    pub orientation: NeedleOrientation,
    #[serde(default)]
    pub coordinate_frame: Option<String>,
}

#[derive(Deserialize)]
pub struct OffsetParams {
    x: f32,
    y: f32,
    z: f32,
    ptm: [f32; 16],
}

pub async fn needle_point_offset(
    State(state): State<ServerState>,
    Json(params): Json<OffsetParams>,
) -> Result<Json<([f32; 3], [f32; 16])>, StatusCode> {
    log::info!(
        "needle_point_offset: offset=({:.3},{:.3},{:.3}) ptm_cols={:?}",
        params.x, params.y, params.z,
        params.ptm
    );
    state.set_offset([params.x, params.y, params.z]);
    state.set_ptm(Mat4::from_cols_array(&params.ptm));
    Ok(Json((state.get_offset(), state.get_ptm().to_cols_array())))
}

pub async fn upload_needle_params(
    State(state): State<ServerState>,
    ConnectInfo(peer): ConnectInfo<std::net::SocketAddr>,
    parts: axum::http::request::Parts,
    body: Option<Json<UploadNeedleJson>>,
) -> StatusCode {
    let started = std::time::Instant::now();
    let (pos, dir, id) = if let Some(Json(b)) = body {
        (
            (b.position.x, b.position.y, b.position.z),
            (b.orientation.a, b.orientation.b, b.orientation.c),
            b.frame_sequence.unwrap_or(0),
        )
    } else {
        return StatusCode::BAD_REQUEST;
    };
    
    static LAST_ARRIVAL_MS: std::sync::atomic::AtomicI64 = std::sync::atomic::AtomicI64::new(0);
    let now_ms = chrono::Local::now().timestamp_millis();
    let prev_ms = LAST_ARRIVAL_MS.swap(now_ms, std::sync::atomic::Ordering::Relaxed);
    let gap_ms = now_ms - prev_ms;
    log::debug!("Needle JSON from {peer}: pos: {:?}, dir: {:?}, gap={}ms", pos, dir, gap_ms);
    if prev_ms != 0 && gap_ms > 500 {
        log::warn!("Needle stream gap {gap_ms}ms from {peer} — request arrived late (send-side or network)");
    }

    // Keep-alive diagnostic: the server (hyper) keeps HTTP/1.1 connections open by
    // default; if the needle device opens a new TCP conn per POST it is client-side.
    let conn_hdr = parts.headers.get(axum::http::header::CONNECTION).and_then(|v| v.to_str().ok()).unwrap_or("").to_ascii_lowercase();
    let reusable = conn_hdr.contains("keep-alive")|| (parts.version >= axum::http::Version::HTTP_11 && !conn_hdr.contains("close"));
    static KA_WARNED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
    if !reusable {
        if !KA_WARNED.swap(true, std::sync::atomic::Ordering::Relaxed) {
            log::warn!(
                "{peer} sends non-reusable HTTP ({:?}, Connection: {:?}) — \
                 client forces a new TCP connection per POST",
                parts.version, conn_hdr
            );
        }
        log::debug!("{peer} non-reusable HTTP ({:?}, Connection: {:?})", parts.version, conn_hdr);
    }

    let len = 400.0;
    let rx = dir.0.to_radians();
    let ry = dir.1.to_radians();
    let rot_y = Mat3::from_rotation_y(ry);
    let rot_x = Mat3::from_rotation_x(rx);

    let dir_m = rot_y * rot_x * Vec3::new(0.0, 0.0, -1.0);
    let tip_m = Vec3::new(pos.0, pos.1, pos.2);
    let end_m = tip_m + dir_m * len;
    debug_assert!(((tip_m - end_m).length() - len).abs() < 0.001);

    let ptm = state.get_ptm();
    log::info!("upload_needle_params: using ptm_cols={:?}", ptm.to_cols_array());

    let axis_transform  = Mat4::from_cols(
        Vec4::new(1.0, 0.0, 0.0, 0.0),
        Vec4::new(0.0, 0.0, 1.0, 0.0),
        Vec4::new(0.0, 1.0, 0.0, 0.0),
        Vec4::new(0.0, 0.0, 0.0, 1.0),
    );

    let end_t = axis_transform.transform_point3(end_m);
    let tip_t = axis_transform.transform_point3(tip_m);

    let offset = state.get_offset();
    let offset = Mat4::from_translation(Vec3::new(offset[0], offset[1], offset[2]));
    let entry_p = offset.transform_point3(ptm.transform_point3(end_t));
    let tip_p   = offset.transform_point3(ptm.transform_point3(tip_t));

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
        "Needle params from {peer}: entry: {:?}, tip: {:?} (handler took {:.3} ms)",
        entry_p, tip_p, started.elapsed().as_secs_f32() * 1000.0
    );

    StatusCode::OK
}

/// Upload a volume to the server (MHA binary data in POST body)
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

    while let Some(field) = multipart.next_field().await.map_err(|e| {
        (StatusCode::BAD_REQUEST, format!("Multipart parse error: {}", e))
    })? {
        let name = field.name().unwrap_or("").to_string();
        let bytes = field.bytes().await.map_err(|e| {
            (StatusCode::BAD_REQUEST, format!("Field read error: {}", e))
        })?;

        if name == "mha" {
            mha_bytes = Some(bytes);
        } else if name == "data" {
            data_bytes = Some(bytes);
        }
    }

    let mha_bytes = mha_bytes
        .ok_or_else(|| (StatusCode::BAD_REQUEST, "Missing 'mha' multipart field".to_string()))?;
    let data_bytes = data_bytes.unwrap_or_default();

    log::info!("upload_volume called with mha={} bytes, data={} bytes", mha_bytes.len(), data_bytes.len());

    let id = generate_uid();
    let now = chrono::Local::now();

    // Persist the MHA to disk by reference (no copy). `Bytes` derefs to `&[u8]`,
    // which `tokio::fs::write` accepts via `AsRef<[u8]>`.
    let mha_disk_path = {
        let path = state.series_path(&id);
        if let Err(e) = tokio::fs::write(&path, &mha_bytes).await {
            log::error!(
                "Failed to persist MHA for volume {} to {:?}: {}",
                id,
                path,
                e
            );
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to persist MHA to {}: {}", path.display(), e),
            ));
        }
        log::info!("Persisted MHA for volume {} ({} bytes)", id, mha_bytes.len());
        path
    };

    // Only one copy of the MHA buffer is made here, when we move it into the
    // owned `Vec<u8>` that `StoredVolume` requires.
    let stored = StoredVolume {
        id: id.clone(),
        mhx_path: Some(mha_bytes.to_vec()),
        data_path: if data_bytes.is_empty() { None } else { Some(data_bytes.to_vec()) },
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
    // Move `stored` directly into the store instead of cloning. After this call
    // we never read `stored` again, so a full deep clone (which would copy
    // every byte of the MHA payload) is unnecessary.
    let series_uid = state.store_raw_volume(stored).await;
    let count_after = state.volume_count().await;
    broadcast_message(&state, WsMessage::ServerStatus {
        loaded_volumes: count_after,
        message: format!("Volume {} uploaded", id),
    });

    log::info!("Volume {} stored ({} -> {} volumes)", id, count_before, count_after);
    Ok(Json(series_uid))
}

pub async fn build_ct_dicom_axum(
    State(state): State<ServerState>,
    Path(id): Path<String>,
) -> Result<Json<StoredVolume>, (StatusCode, String)> {
    let stored = state.get_volume(&id).await.ok_or_else(|| (StatusCode::NOT_FOUND, format!("Volume {} not found", id)))?;
    log::info!("Volume {} found: id={}, mhx_path_size={}, data_path_size={}", 
        id, 
        stored.id, 
        stored.mhx_path.as_ref().map(|p| p.len()).unwrap_or(0),
        stored.data_path.as_ref().map(|p| p.len()).unwrap_or(0)
    );

    let mhx_path = stored.mhx_path.as_ref().ok_or_else(|| (StatusCode::BAD_REQUEST, "MHX path is missing".to_string()))?;
    let data_path = stored.data_path.as_ref().filter(|d| !d.is_empty()).map(|v| v.as_slice());
    let out_dir = std::path::PathBuf::from("C:\\user\\dicoms");
    let temp_dir = out_dir.join(format!("CT_{}", stored.id));
    
    std::fs::create_dir_all(&temp_dir).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to create temp dir: {}", e)))?;
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
    ).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to build DICOM: {}", e)))?;
    log::info!("DICOM exported to {:?}", temp_dir);
    Ok(Json(stored))
}

pub async fn upload_obj(mut multipart: Multipart) -> Result<Response<Body>, StatusCode> {
    let mut file_bytes = None;
    let mut x = None;
    let mut y = None;
    let mut z = None;

    while let Some(field) = multipart.next_field().await.map_err(|e| {
        eprintln!("multipart error: {:?}", e);
        StatusCode::BAD_REQUEST
    })?{
        let name = field.name().unwrap_or("");
        match name {
            "file" => {
                file_bytes  = Some(field.bytes().await.map_err(|e| {
                    eprintln!("read upload error: {:?}", e);
                    StatusCode::BAD_REQUEST
                })?);
            }
            "x" => {
                let text = field.text().await.map_err(|e| {
                    eprintln!("read field error: {:?}", e);
                    StatusCode::BAD_REQUEST
                })?;
                x = Some(text.parse::<f32>().map_err(|_| StatusCode::BAD_REQUEST)?);
            }
            "y" => {
                let text = field.text().await.map_err(|e| {
                    eprintln!("read field error: {:?}", e);
                    StatusCode::BAD_REQUEST
                })?;
                y = Some(text.parse::<f32>().map_err(|_| StatusCode::BAD_REQUEST)?);
            }
            "z" => {
                let text = field.text().await.map_err(|e| {
                    eprintln!("read field error: {:?}", e);
                    StatusCode::BAD_REQUEST
                })?;
                z = Some(text.parse::<f32>().map_err(|_| StatusCode::BAD_REQUEST)?);
            }
            _ => {
                eprintln!("Unknown field: {}", name);
                continue;
            }
        }
    }

    let bytes = file_bytes.ok_or(StatusCode::BAD_REQUEST)?;
    let volume_size_mm = Vec3::new(
        x.ok_or(StatusCode::BAD_REQUEST)?,
        y.ok_or(StatusCode::BAD_REQUEST)?,
        z.ok_or(StatusCode::BAD_REQUEST)?,
    );

    let path = std::env::temp_dir().join(format!("{}.obj", uuid::Uuid::new_v4()));
    tokio::fs::write(&path, &bytes).await.map_err(|e| {
        eprintln!("write failed: {:?}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let meshes = Mesh::import_obj(path.to_str().unwrap(), volume_size_mm).map_err(|e| {
        eprintln!("OBJ import failed: {:?}", e);
        StatusCode::BAD_REQUEST
    })?;

    let bin = bincode::serialize(&meshes).map_err(|e| {
        eprintln!("serialize failed: {:?}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Response::builder()
        .header("Content-Type", "application/octet-stream")
        .body(Body::from(bin))
        .unwrap())
}

pub async fn start_segmentation(
    State(state): State<ServerState>,
    Json(req): Json<SegmentRequest>,
) -> Result<Json<SegmentResponse>, (StatusCode, String)> {
    let series = req.series_id.clone();
    let model = req.model.clone();
    match ai_handler::handle_segment(state.clone(), model, series).await {
        Ok(resp) => Ok(Json(resp)),
        Err(e) => {
            log::error!("start_segmentation failed: {e}");
            broadcast_message(&state, WsMessage::Error {
                message: format!("start_segmentation failed: {e}"),
            });
            Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
        }
    }
}

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
        dimensions: (0, 0, 0), // dimensions are not tracked in AiTask; clients should rely on the original CT volume.
    }))
}

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

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Circle {
    pub width: f32,
    pub height: f32,
    pub x1: f32,
    pub y1: f32,
    pub radius_1: f32,
    pub x2: f32,
    pub y2: f32,
    pub radius_2: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalibrationResult {
    pub c: [f64; 5],
    pub a: [f64; 5],
    pub b: [[f64; 5]; 5],
    pub condition_number: f64,
    pub determinant: f64,
    pub singular_values: [f64; 5],
}

pub const WATER_MU: f32 = 0.27;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaskData {
    pub w: Vec<f32>,
    pub t: Vec<f32>,
    pub mask: Vec<u8>,
    pub water_mask: Vec<bool>,
    pub air_mask: Vec<bool>,
}

pub async fn handle_load_slice(
    socket: &mut WebSocket,
    state: &ServerState,
    slice: usize,
    width: usize,
    height: usize
) {
    let data = match state.get_ct_slice(slice) {
        Ok(data) => data,
        Err(err) => {
            send_error(socket, err.to_string()).await;
            return;
        }
    };
    let message = WsMessage::Slice { slice, width, height, data };
    send_json(socket, &message).await;
}

/// Build a binary mask from the user's confirmed circle and stash it in
/// `ServerState::mask_data`. Returns the mask so the caller can also
/// forward it to the calibration stage.
fn build_circle_mask(width: usize, height: usize, circle: Circle) -> MaskData {
    let len = width * height;
    let mut w = vec![0.0f32; len];
    let mut t = vec![0.0f32; len];
    let mut mask = vec![0u8; len];
    let mut water_mask = vec![false; len];
    let mut air_mask = vec![false; len];
    let water_radius = circle.radius_1;
    let air_radius = circle.radius_2;
    let water_radius2 = water_radius * water_radius;
    let air_radius2 = air_radius * air_radius;
    for y in 0..height {
        for x in 0..width {
            let dx = x as f32 - circle.x1;
            let dy = y as f32 - circle.y1;
            let dist2 = dx * dx + dy * dy;
            let index = y * width + x;

            if dist2 <= water_radius2 {
                water_mask[index] = true;
                w[index] = 1.0;
                t[index] = WATER_MU;
                mask[index] = 1;
            }

            if dist2 >= water_radius2 && dist2 <= air_radius2 {
                air_mask[index] = true;
                w[index] = 1.0;
                t[index] = 0.0;
                mask[index] = 2;
            }
        }
    }

    MaskData { w, t, mask, water_mask, air_mask }
}

/// Serialize a `WsMessage` to JSON and push it to the client as a
/// text WebSocket frame. Centralises the (de)serialization so every
/// handler emits the same wire format.
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

/// Convenience: send an `Error` message to the client.
pub async fn send_error(socket: &mut WebSocket, message: String) {
    send_json(socket, &WsMessage::Error { message }).await;
}

/// Handle `ConfirmCircle { slice, circle }` from the client:
/// stash the circle (and the mask it implies) in `ServerState`, then
/// acknowledge the client.
pub async fn handle_confirm_circle(
    socket: &mut WebSocket,
    state: &ServerState,
    slice: usize,
    circle: Circle,
) {
    // Build a mask from the confirmed circle. We use the circle's own
    // (width, height) for the working buffer; the slice index is just
    // metadata carried over from the client.
    let width = circle.width as usize;
    let height = circle.height as usize;
    let mask = build_circle_mask(width, height, circle);
    state.set_mask_data(Some(mask));
    state.set_mask_circle(Some(circle));

    send_json(
        socket,
        &WsMessage::CircleAccepted { slice, circle },
    )
    .await;
}

/// Handle `RunCalibration` from the client. The actual computation is
/// spawned onto a Tokio task so that the WebSocket receive loop keeps
/// processing other commands while the calibration runs.
pub async fn handle_run_calibration(socket: &mut WebSocket, state: &ServerState) {
    // Pull the current mask / circle out of shared state up front so the
    // spawned task can run with a self-contained snapshot.
    let (circle, mask) = match (state.mask_circle(), state.mask_data()) {
        (Some(c), Some(m)) => (c, m),
        _ => {
            send_error(
                socket,
                "no confirmed circle yet — please confirm a circle first".to_string(),
            )
            .await;
            return;
        }
    };

    // Acknowledge that the request has been accepted.
    send_json(
        socket,
        &WsMessage::Progress {
            stage: "Starting calibration".to_string(),
            progress: 0.0,
        },
    )
    .await;

    let tx = state.ws_tx.clone();
    tokio::spawn(async move {
        run_calibration_pipeline(tx, circle, mask).await;
    });
}

/// Calibration pipeline executed on a background Tokio task. The real
/// algorithm lives behind `compute_calibration` below; this wrapper just
/// turns its progress into `WsMessage::Progress` updates on the broadcast
/// channel so every connected client can observe them.
async fn run_calibration_pipeline(
    tx: broadcast::Sender<WsMessage>,
    circle: Circle,
    mask: MaskData,
) {
    let stages: &[(&str, f32)] = &[
        ("Creating masks", 0.1),
        ("Computing a", 0.3),
        ("Computing B", 0.5),
        ("SVD", 0.8),
    ];

    for (stage, progress) in stages {
        let _ = tx.send(WsMessage::Progress {
            stage: (*stage).to_string(),
            progress: *progress,
        });
        // Cooperative yield so progress messages flush promptly even if
        // each stage is currently a no-op stub.
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }

    match compute_calibration(&circle, &mask) {
        Ok(result) => {
            let _ = tx.send(WsMessage::CalibrationResult { result });
        }
        Err(e) => {
            log::error!("calibration failed: {e}");
            let _ = tx.send(WsMessage::Error {
                message: format!("calibration failed: {e}"),
            });
        }
    }

    let _ = tx.send(WsMessage::Finished);
}

/// Placeholder for the actual CBCT calibration. Returns a zero-initialised
/// result so the wire protocol is fully exercised end-to-end. The real
/// algorithm (water/air regression, SVD, etc.) will replace this body in
/// a follow-up change.
fn compute_calibration(circle: &Circle, _mask: &MaskData) -> Result<CalibrationResult, String> {
    log::info!(
        "compute_calibration(stub): circle=(x1={:.2}, y1={:.2}, r1={:.2}, x2={:.2}, y2={:.2}, r2={:.2})",
        circle.x1, circle.y1, circle.radius_1, circle.x2, circle.y2, circle.radius_2
    );
    Ok(CalibrationResult {
        c: [0.0; 5],
        a: [0.0; 5],
        b: [[0.0; 5]; 5],
        condition_number: 0.0,
        determinant: 0.0,
        singular_values: [0.0; 5],
    })
}