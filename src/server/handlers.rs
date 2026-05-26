use axum::{
    extract::{Multipart, Path, Query, State, WebSocketUpgrade},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

use crate::data::dicom::{build_ct_dicom, FsSink, generate_uid, Patient, StudySet};
use crate::server::state::{ServerState, StoredVolume};
use crate::server::ws::WsMessage;

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
                            if let Ok(client_msg) = serde_json::from_str::<ClientMessage>(&text) {
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
    let stored = StoredVolume {
        id: id.clone(),
        mhx_path: Some(mha_bytes),
        data_path: Some(data_bytes),
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
    log::info!("build_ct_dicom called with {:?}", id);

    let stored = state.get_volume(&id).await
        .ok_or_else(|| (StatusCode::NOT_FOUND, format!("Volume {} not found", id)))?;
    log::info!("Volume {} found: id={}, mhx_path_size={}, data_path_size={}", 
        id, 
        stored.id, 
        stored.mhx_path.as_ref().map(|p| p.len()).unwrap_or(0),
        stored.data_path.as_ref().map(|p| p.len()).unwrap_or(0));

    let mhx_path = stored.mhx_path.as_ref()
        .ok_or_else(|| (StatusCode::BAD_REQUEST, "MHX path is missing".to_string()))?;
    let data_path = stored.data_path.as_ref()
        .ok_or_else(|| (StatusCode::BAD_REQUEST, "Data path is missing".to_string()))?;

    let out_dir = std::path::PathBuf::from("C:\\user\\dicoms");
    let temp_dir = out_dir.join(format!("CT_{}", stored.id));
    std::fs::create_dir_all(&temp_dir).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to create temp dir: {}", e)))?;

    let mut sink = FsSink {
        out_dir: temp_dir.clone(),
    };
    
    build_ct_dicom(
        mhx_path,
        Some(data_path),
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
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to build DICOM: {}", e)))?;

    log::info!("DICOM exported to {:?}", temp_dir);
    Ok(Json(stored))
}
