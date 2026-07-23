use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use serde::Serialize;
use tokio::sync::{broadcast, RwLock};

use crate::server::ai::AiService;
use crate::server::ai_task::TaskManager;
use crate::server::ws::WsMessage;
use crate::data::dicom::Patient;
use crate::data::dicom::StudySet;

/// Represents a stored volume in server memory
#[derive(Clone, Serialize)]
pub struct StoredVolume {
    /// Unique identifier for this volume
    pub id: String,
    /// The CT volume data
    #[serde(skip_serializing)]
    pub mhx_path: Option<Vec<u8>>,
    pub data_path: Option<Vec<u8>>,
    /// Absolute path on disk where the MHA file for this volume is persisted.
    /// The Python AI service (`src/server/python/app.py`) reads from this
    /// path; we populate it in `upload_volume` so that the same id is
    /// usable as a `series_id` for `/api/segment` without any extra
    /// round-trip.
    #[serde(skip_serializing)]
    pub mha_disk_path: Option<PathBuf>,
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
    /// Whether to actually write MHA files to `series_dir`. 
    pub persist_mha_to_disk: bool,
}

impl ServerState {
    pub fn new() -> Self {
        let (ws_tx, _) = broadcast::channel(256);

        // Resolve the on-disk series directory
        let series_dir_from_env = Some(PathBuf::from("C:/user/kepler_series"));

        let (series_dir, using_fallback) = match series_dir_from_env {
            Some(p) => {
                (p, false)
            },
            None => (std::env::temp_dir().join("kepler_series"), true),
        };

        log::info!("Persisting uploaded MHAs under {:?}", series_dir);

        if using_fallback {
            eprintln!(
                "\n\
                 ============================================================\n\
                 [kepler-wgpu] KEPLER_SERIES_DIR is not set.\n\
                 [kepler-wgpu]   Uploaded volumes will be kept in memory only\n\
                 [kepler-wgpu]   and NOT persisted to disk.\n\
                 [kepler-wgpu]   The Python AI service (src/server/python/app.py)\n\
                 [kepler-wgpu]   will not be able to read MHA files for\n\
                 [kepler-wgpu]   segmentation.\n\
                 [kepler-wgpu]\n\
                 [kepler-wgpu]   If you need AI segmentation, set\n\
                 [kepler-wgpu]     $env:KEPLER_SERIES_DIR = \"path\\\\to\\\\share\"\n\
                 [kepler-wgpu]   on BOTH the Rust and Python terminals.\n\
                 ============================================================\n"
            );
            log::warn!(
                "KEPLER_SERIES_DIR not set; uploaded volumes stay in memory only. \
                 Set KEPLER_SERIES_DIR on both Rust and Python sides if you need AI segmentation."
            );
        }

        let persist_mha_to_disk = !using_fallback;

        Self {
            ai: Arc::new(AiService::new()),
            tasks: TaskManager::new(),
            volumes: Arc::new(RwLock::new(HashMap::new())),
            start_time: chrono::Local::now().format("%Y-%m-%dT%H:%M:%S").to_string(),
            ws_tx,
            series_dir,
            persist_mha_to_disk,
        }
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
}

impl Default for ServerState {
    fn default() -> Self {
        Self::new()
    }
}
