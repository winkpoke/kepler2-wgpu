use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use parking_lot::Mutex;
use serde::Serialize;
use tokio::sync::{broadcast, RwLock};
use axum::body::Bytes;

use crate::server::ai::AiService;
use crate::server::ai_task::TaskManager;
use crate::server::navcomputer::NavState;
use crate::server::segment_engine::ModelManager;
use crate::server::ws::WsMessage;
use crate::data::dicom::Patient;
use crate::data::dicom::StudySet;
use crate::data::ct_volume::CTVolume;
use crate::gpu::GpuState;
use crate::device::hv::HvDevice;
use crate::device::plc_client::PlcHandle;

/// Represents a stored volume in server memory
#[derive(Clone, Serialize)]
pub struct StoredVolume {
    /// Unique identifier for this volume
    pub id: String,
    /// The CT volume data
    #[serde(skip_serializing)]
    pub mhx_path: Option<Bytes>,
    #[serde(skip_serializing)]
    pub data_path: Option<Bytes>,
    /// Absolute path on disk where the MHA file for this volume is persisted
    #[serde(skip_serializing)]
    pub mha_disk_path: PathBuf,
    /// Timestamp when volume was loaded (ISO 8601)
    pub loaded_at: String,
    pub patient: Patient,
    pub study: StudySet,
    pub kv: f64,
    pub m_as: f64,
    pub slope: f32,
    pub intercept: f32,
    pub patient_position: String,
    pub modality: String,
}

/// Shared application state for the Axum server
#[derive(Clone)]
pub struct ServerState {
    /// AI service for segmenting volumes
    pub ai: Arc<AiService>,
    /// Task manager that tracks the lifecycle of every segmentation task.
    pub tasks: TaskManager,
    /// Map of volume ID -> StoredVolume
    pub volumes: Arc<RwLock<HashMap<String, StoredVolume>>>,
    /// Server start time (ISO 8601)
    pub start_time: String,
    /// Broadcast channel for WebSocket events (capacity: 256)
    pub ws_tx: broadcast::Sender<WsMessage>,
    /// Directory where uploaded MHA files 
    pub series_dir: PathBuf,
    pub output_root: PathBuf,
    /// offset for the entry and tip points in the patient coordinate system
    pub offset: Arc<Mutex<[f32; 3]>>,
    pub ptm: Arc<Mutex<glam::Mat4>>,
    /// Cached CT volume (used by the GPU volume smoke-test endpoint).
    pub ct_volume: Arc<Mutex<Option<CTVolume>>>,
    /// Navigation Computer 桥接状态（基地址 + 已接收回调）
    pub nav: Arc<NavState>,
    /// ONNX model registry
    pub onnx_models: Arc<Mutex<Option<Arc<ModelManager>>>>,
    /// Native GPU backend (wgpu 30) for future compute workloads. Lazily initialized.
    pub gpu: Arc<GpuState>,
    pub hv: Arc<Mutex<HvDevice>>,
    /// PLC Modbus TCP 客户端
    pub plc: PlcHandle,
}

impl ServerState {
    pub fn new() -> Self {
        let (ws_tx, _) = broadcast::channel(256);

        // Resolve the on-disk series directory
        let series_dir = PathBuf::from("C:/user/kepler_series");
        if series_dir.exists() {
            if let Err(e) = std::fs::remove_dir_all(&series_dir) {
                log::warn!("failed to clean series dir {:?}: {}", series_dir, e);
            }
        }
        if let Err(e) = std::fs::create_dir_all(&series_dir) {
            log::error!("failed to create series dir {:?}: {}", series_dir, e);
        }

        let output_root = PathBuf::from("C:/user/kepler_ai_output");
        if let Err(e) = std::fs::create_dir_all(&output_root) {
            log::error!("failed to create output root {:?}: {}", output_root, e);
        }

        let port = std::env::var("HV_PORT").unwrap_or_else(|_| "COM6".to_string());
        let baud = std::env::var("HV_BAUD").ok().and_then(|s| s.parse().ok()).unwrap_or(19200);

        let mut hv = HvDevice::new();
        if let Err(e) = hv.connect(&port, baud) {
            println!("error connect {port} hv device: {e}");
        }
    
        Self {
            ai: Arc::new(AiService::new()),
            tasks: TaskManager::new(),
            volumes: Arc::new(RwLock::new(HashMap::new())),
            start_time: chrono::Local::now().format("%Y-%m-%dT%H:%M:%S").to_string(),
            ws_tx,
            series_dir,
            output_root,
            offset: Arc::new(Mutex::new([0.0, 0.0, 0.0])),
            ptm: Arc::new(Mutex::new(glam::Mat4::IDENTITY)),
            ct_volume: Arc::new(Mutex::new(None)),
            nav: Arc::new(NavState::new()),
            onnx_models: Arc::new(Mutex::new(None)),
            gpu: Arc::new(GpuState::new()),
            hv: Arc::new(Mutex::new(hv)),
            plc: crate::device::plc_client::PlcClient::from_env(),
        }
    }

    pub fn set_offset(&self, offset: [f32; 3]) {
        *self.offset.lock() = offset;
    }

    pub fn set_ptm(&self, ptm: glam::Mat4) {
        *self.ptm.lock() = ptm;
    }

    pub fn get_offset(&self) -> [f32; 3] {
        *self.offset.lock()
    }

    pub fn get_ptm(&self) -> glam::Mat4 {
        *self.ptm.lock()
    }

    /// Store a new volume from a raw CTVolume and return its ID
    pub async fn store_raw_volume(&self, volume: StoredVolume) -> String {
        let id = volume.id.clone();
        self.volumes.write().await.insert(id.clone(), volume);
        id
    }

    /// Get a volume by ID
    pub async fn get_volume(&self, id: &String) -> Option<StoredVolume> {
        self.volumes.read().await.get(id).cloned()
    }

    /// Get volume count
    pub async fn volume_count(&self) -> usize {
        self.volumes.read().await.len()
    }

    /// Absolute on-disk path where the MHA file for `series_id` should
    /// live. This matches the convention in
    /// `src/server/python/app.py::_run_task`:
    ///
    /// ```python
    /// series_path = os.path.join(SERIES_DIR, f"{series_id}.mha")
    /// ```
    pub fn series_path(&self, series_id: &str) -> PathBuf {
        self.series_dir.join(format!("{}.mha", series_id))
    }

    /// Cache the most recent CT volume.
    pub fn set_ct_volume(&self, volume: CTVolume) {
        *self.ct_volume.lock() = Some(volume);
    }

    /// Returns the cached CT volume (if any).
    pub fn ct_volume(&self) -> Option<CTVolume> {
        self.ct_volume.lock().clone()
    }
}

impl Default for ServerState {
    fn default() -> Self {
        Self::new()
    }
}
