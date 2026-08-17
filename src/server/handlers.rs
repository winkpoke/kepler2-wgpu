use axum::{
    extract::{Multipart, Path, Query, State, WebSocketUpgrade},
    http::StatusCode,
    response::IntoResponse,
    Json,
    body::Body,
    response::Response,
};
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;
use bincode;

use crate::server::ai_handler;
use crate::server::ai_model::{CancelRequest, SegmentRequest, SegmentResponse};
use crate::data::dicom::{build_ct_dicom, FsSink, generate_uid, Patient, StudySet};
use crate::server::state::{ServerState, StoredVolume};
use crate::server::ws::WsMessage;
use crate::rendering::view::mesh::Mesh;

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

pub async fn health_check(
    State(state): State<ServerState>,
) -> Json<HealthResponse> {
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

async fn handle_socket(mut socket: axum::extract::ws::WebSocket, state: ServerState) {
    let mut rx = state.ws_tx.subscribe();

    let count = state.volume_count().await;
    let msg = WsMessage::ServerStatus {
        loaded_volumes: count,
        message: "Connected".to_string(),
    };
    if let Ok(json) = serde_json::to_string(&msg) {
        let _ = socket.send(axum::extract::ws::Message::Text(json)).await;
    }

    loop {
        tokio::select! {
            ws_msg = rx.recv() => {
                match ws_msg {
                    Ok(msg) => {
                        if let Ok(json) = serde_json::to_string(&msg) {
                            if socket.send(axum::extract::ws::Message::Text(json)).await.is_err() {
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
                        axum::extract::ws::Message::Text(text) => {
                            if let Ok(cmd) = serde_json::from_str::<crate::server::ws::WsClientCommand>(&text) {
                                match cmd {
                                    crate::server::ws::WsClientCommand::Segment { model, series } => {
                                        log::info!("WS: start segment model={} series={}", model, series);
                                        if let Err(e) = ai_handler::handle_segment(
                                            state.clone(),
                                            model,
                                            series,
                                        )
                                        .await
                                        {
                                            log::error!("WS segment dispatch failed: {e}");
                                        }
                                    }
                                    crate::server::ws::WsClientCommand::SegmentCancel { task_id } => {
                                        log::info!("WS: cancel segment task_id={}", task_id);
                                        if let Err(e) = ai_handler::handle_cancel(
                                            state.clone(),
                                            CancelRequest { task_id: task_id.clone() },
                                        )
                                        .await
                                        {
                                            log::error!("WS segment cancel failed: {e}");
                                        }
                                    }
                                    crate::server::ws::WsClientCommand::SegmentProgressQuery { task_id } => {
                                        if let Some(t) = state.tasks.get(&task_id).await {
                                            let _ = state.ws_tx.send(WsMessage::SegmentProgress {
                                                task_id: task_id.clone(),
                                                value: t.progress,
                                            });
                                        }
                                    }
                                }
                            } else if let Ok(client_msg) = serde_json::from_str::<ClientMessage>(&text) {
                                if client_msg.kind == "ping" {
                                    let hb = WsMessage::Heartbeat {
                                        timestamp: chrono::Local::now().format("%Y-%m-%dT%H:%M:%S").to_string(),
                                    };
                                    if let Ok(json) = serde_json::to_string(&hb) {
                                        if socket.send(axum::extract::ws::Message::Text(json)).await.is_err() {
                                            break;
                                        }
                                    }
                                }
                            }
                        }
                        axum::extract::ws::Message::Ping(bytes) => {
                            if socket.send(axum::extract::ws::Message::Pong(bytes)).await.is_err() {
                                break;
                            }
                        }
                        axum::extract::ws::Message::Close(_) => break,
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

/// 回给调用方的确认信息
#[derive(Serialize)]
pub struct NeedleEcho {
    pub status: String,
    pub id: u32,
    pub pos: (f32, f32, f32),
    pub dir: (f32, f32, f32),
    pub len_mm: f32,
    pub tip: (f32, f32, f32),
}

pub async fn upload_needle_params(
    State(state): State<ServerState>,
    body: Option<Json<UploadNeedleJson>>,
) -> Result<Json<NeedleEcho>, StatusCode> {
    let (pos, dir, id) = if let Some(Json(b)) = body {
        log::info!(
            "Needle JSON: frame_sequence: {:?}, coordinate_frame: {:?}, pos_unit: {:?}, dir_unit: {:?}",
            b.frame_sequence, b.coordinate_frame, b.position.unit, b.orientation.unit
        );
        (
            (b.position.x, b.position.y, b.position.z),
            (b.orientation.a, b.orientation.b, b.orientation.c),
            b.frame_sequence.unwrap_or(0),
        )
    } else {
        return Err(StatusCode::BAD_REQUEST);
    };

    let len = 400.0;
    let rx = dir.0.to_radians();
    let ry = dir.1.to_radians();
    let rz = dir.2.to_radians();

    let rot = glam::Mat3::from_euler(
        glam::EulerRot::XYZ,
        rx,
        ry,
        rz,
    );

    let dir_vec = (rot * glam::Vec3::Z).normalize();

    let tip = glam::Vec3::new(pos.0, pos.1, pos.2);
    let entry = tip - dir_vec * len;

    debug_assert!(
        (tip - entry).length() - len < 0.001,
    );

    let entry = (entry.x + 256.0, -entry.z + 306.0, - entry.y + 256.0);
    let tip = (tip.x + 256.0, -tip.z + 306.0, - tip.y + 256.0);

    log::info!(
        "Needle params: entry: {:?}, dir: {:?}, len: {:.1}mm, tip: {:?}",
        entry, dir, len, tip
    );

    // Broadcast to every connected browser so the wasm renderer can draw the needle.
    let _ = state.ws_tx.send(WsMessage::NeedleSet {
        id,
        x: entry.0,
        y: entry.1,
        z: entry.2,
        lx: tip.0,
        ly: tip.1,
        lz: tip.2,
        r: 0.2,
        g: 0.9,
        b: 0.2,
        dir,
        len_mm: len,
    });

    Ok(Json(NeedleEcho {
        status: "ok".to_string(),
        id,
        pos: entry,
        dir,
        len_mm: len,
        tip,
    }))
}

/// Upload a volume to the server (MHA binary data in POST body)
pub async fn upload_volume(
    State(state): State<ServerState>,
    Query(params): Query<UploadParams>,
    mut multipart: Multipart,
) -> Result<Json<String>, (StatusCode, String)> {
    let mut mha_bytes: Vec<u8> = Vec::new();
    let mut data_bytes: Vec<u8> = Vec::new();

    while let Some(field) = multipart.next_field().await.map_err(|e| {
        (StatusCode::BAD_REQUEST, format!("Multipart parse error: {}", e))
    })? {
        let name = field.name().unwrap_or("").to_string();
        let bytes = field.bytes().await.map_err(|e| {
            (StatusCode::BAD_REQUEST, format!("Field read error: {}", e))
        })?;
        
        if name == "mha" {
            mha_bytes = bytes.to_vec();
        } else if name == "data" {
            data_bytes = bytes.to_vec();
        }
    }

    log::info!("upload_volume called with mha={} bytes, data={} bytes", mha_bytes.len(), data_bytes.len());

    let id = generate_uid();
    let now = chrono::Local::now();

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

    let stored = StoredVolume {
        id: id.clone(),
        mhx_path: Some(mha_bytes),
        data_path: Some(data_bytes),
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
    let series_uid = state.store_raw_volume(stored.clone()).await;
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
    while let Some(field) = multipart.next_field().await.map_err(|e| {
        eprintln!("multipart error: {:?}", e);
        StatusCode::BAD_REQUEST
    })?{
        if field.name() == Some("file") {
            let bytes = field.bytes().await.map_err(|e| {
                eprintln!("read upload error: {:?}", e);
                StatusCode::BAD_REQUEST
            })?;
            let path = std::env::temp_dir().join(format!("{}.obj", uuid::Uuid::new_v4()));
            tokio::fs::write(&path, &bytes).await.map_err(|e| {
                eprintln!("write failed: {:?}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?;
            let meshes = Mesh::import_obj(path.to_str().unwrap()).map_err(|e| {
                eprintln!("OBJ import failed: {:?}", e);
                StatusCode::BAD_REQUEST
            })?;
            let bin = bincode::serialize(&meshes).map_err(|e| {
                eprintln!("serialize failed: {:?}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?;
            return Ok(
                Response::builder()
                    .header(
                        "Content-Type",
                        "application/octet-stream"
                    )
                    .body(Body::from(bin))
                    .unwrap()
            );
        }
    }
    Err(StatusCode::BAD_REQUEST)
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