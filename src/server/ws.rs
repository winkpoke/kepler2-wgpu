use serde::Serialize;

/// WebSocket message types sent from server to client
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
}
