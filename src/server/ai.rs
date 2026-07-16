use anyhow::Result;
use base64::Engine;
use reqwest::Client;

use crate::server::ai_model::{
    AiServiceTask, SegmentRequest, SegmentResponse,
};

/// HTTP client for the external Python AI service (e.g. TotalSegmentator).
///
/// The Rust server is intentionally a thin proxy: segmentation is submitted
/// here, progress is polled from here, and the resulting mask is downloaded
/// from here. The server is fully decoupled from the rendering engine.
#[cfg(not(target_arch = "wasm32"))]
pub struct AiService {
    client: Client,
    base_url: String,
}

#[cfg(not(target_arch = "wasm32"))]
impl AiService {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
            base_url: std::env::var("KEPLER_AI_URL")
                .unwrap_or_else(|_| "http://localhost:8001".into()),
        }
    }

    /// Submit a segmentation request and return the immediate acknowledgement
    /// (containing the new `task_id`).
    pub async fn segment(&self, req: SegmentRequest) -> Result<SegmentResponse> {
        let resp = self
            .client
            .post(format!("{}/segment", self.base_url))
            .json(&req)
            .send()
            .await?
            .json::<SegmentResponse>()
            .await?;
        Ok(resp)
    }

    /// Poll the AI service for the current state of a task.
    pub async fn progress(&self, task_id: &str) -> Result<AiServiceTask> {
        let resp = self
            .client
            .get(format!("{}/task/{}", self.base_url, task_id))
            .send()
            .await?
            .json::<AiServiceTask>()
            .await?;
        Ok(resp)
    }

    /// Ask the AI service to cancel a running task. The service is expected
    /// to flip the task state to `cancelled` and stop further work.
    pub async fn cancel(&self, task_id: &str) -> Result<AiServiceTask> {
        let resp = self
            .client
            .post(format!("{}/task/{}/cancel", self.base_url, task_id))
            .send()
            .await?
            .json::<AiServiceTask>()
            .await?;
        Ok(resp)
    }

    /// Download the raw mask bytes for a completed task. The Python service
    /// returns a base64-encoded mask in the task payload; this helper decodes
    /// it and returns the raw `Vec<u8>` for upload to the GPU.
    pub async fn download(&self, task_id: &str) -> Result<Vec<u8>> {
        let task = self.progress(task_id).await?;
        let b64 = task
            .mask_base64
            .ok_or_else(|| anyhow::anyhow!("AI service returned no mask for task {}", task_id))?;
        let bytes = base64::engine::general_purpose::STANDARD.decode(b64)?;
        Ok(bytes)
    }

    /// Fetch the most recent cached spine mask for a given series_id.
    ///
    /// Hits the Python service's `GET /cached_mask/{series_id}` endpoint,
    /// which scans OUTPUT_ROOT for any task directory containing a
    /// `spine.nii.gz` file and returns the most recent one. Returns the
    /// raw JSON response so the caller can inspect `status` (ok / missing
    /// / error) and the base64-encoded `mask_base64`.
    pub async fn cached_mask(&self, series_id: &str) -> Result<serde_json::Value> {
        let resp = self
            .client
            .get(format!("{}/cached_mask/{}", self.base_url, series_id))
            .send()
            .await?
            .json::<serde_json::Value>()
            .await?;
        Ok(resp)
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl Default for AiService {
    fn default() -> Self {
        Self::new()
    }
}
