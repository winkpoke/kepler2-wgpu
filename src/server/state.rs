use std::collections::HashMap;
use std::sync::Arc;
use serde::Serialize;
use tokio::sync::{broadcast, RwLock};

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
    /// Map of volume ID -> StoredVolume
    pub volumes: Arc<RwLock<HashMap<String, StoredVolume>>>,
    /// Server start time (ISO 8601)
    pub start_time: String,
    /// Broadcast channel for WebSocket events (capacity: 256)
    pub ws_tx: broadcast::Sender<WsMessage>,
}

impl ServerState {
    pub fn new() -> Self {
        let (ws_tx, _) = broadcast::channel(256);
        Self {
            volumes: Arc::new(RwLock::new(HashMap::new())),
            start_time: chrono::Local::now().format("%Y-%m-%dT%H:%M:%S").to_string(),
            ws_tx,
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
}

impl Default for ServerState {
    fn default() -> Self {
        Self::new()
    }
}
