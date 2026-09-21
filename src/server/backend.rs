#![cfg(not(target_arch = "wasm32"))]
//! Unified, lightweight segmentation backend interface.
//!
//! Both inference paths (ONNX-direct and the embedded Python nnU-Net
//! reference) implement [`SegmentationBackend`] and return the same
//! [`EngineResult`](crate::server::segment_engine::SegmentResult) shape, so
//! the Axum handler and the rest of the server never need to know which
//! backend produced a mask. The ONNX engine (`segment_engine.rs`) is reused
//! verbatim — this module only wraps it.
//!
//! Avoid introducing heavy new dependencies: the trait is async via the
//! already-present `async-trait` crate, and the ONNX path keeps using
//! `spawn_blocking` exactly as before (ORT sessions are sync). The Python
//! path also uses `spawn_blocking` so the GIL/PyTorch work never blocks the
//! Axum runtime.

use std::sync::Arc;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use crate::server::ai_model::SegmentRequest as ApiSegmentRequest;
use crate::server::segment_engine::{
    ModelManager, SegmentRequest as EngineRequest, SegmentResult as EngineResult,
};

/// Common contract for every segmentation backend.
///
/// `volume` / `shape` carry the Z,Y,X `f32` HU buffer (used by ONNX). The
/// Python backend ignores them and instead reads the on-disk MHA identified
/// by `req.series_id` (the upload flow already persists it), keeping the
/// data interface simple and matching the existing Python behavior.
#[async_trait]
pub trait SegmentationBackend {
    /// Human-readable backend tag, used for logs / dispatch (`"onnx"` / `"python"`).
    fn name(&self) -> &'static str;

    /// Run segmentation and return the multi-label mask + metadata.
    async fn segment(
        &self,
        req: &ApiSegmentRequest,
        volume: Arc<Vec<f32>>,
        shape: (usize, usize, usize),
    ) -> Result<EngineResult>;
}

/// ONNX-direct backend. Wraps the existing [`ModelManager`] so the session
/// cache (`Arc<OnnxSegmentator>` keyed by model name) is reused — a repeated
/// request for the same model must NOT reload the session.
pub struct OnnxBackend {
    mgr: Arc<ModelManager>,
}

impl OnnxBackend {
    pub fn new(mgr: Arc<ModelManager>) -> Self {
        Self { mgr }
    }
}

#[async_trait]
impl SegmentationBackend for OnnxBackend {
    fn name(&self) -> &'static str {
        "onnx"
    }

    async fn segment(
        &self,
        req: &ApiSegmentRequest,
        volume: Arc<Vec<f32>>,
        shape: (usize, usize, usize),
    ) -> Result<EngineResult> {
        log::info!("[AI/ONNX] model={} backend=onnx loading session", req.model);
        let model = self
            .mgr
            .get_model(&req.model)
            .await
            .map_err(|e| anyhow!("load onnx model {}: {e}", req.model))?;

        let engine_req = EngineRequest {
            series_id: req.series_id.clone(),
            model: req.model.clone(),
            fast_mode: true,
        };

        // ORT inference is sync and GPU/CPU heavy; run it off the tokio
        // worker so we never block the Axum runtime. `volume` is an `Arc`,
        // so moving a clone into the blocking task is cheap.
        log::info!("[AI/ONNX] model={} inference started", req.model);
        let result = tokio::task::spawn_blocking(move || model.segment(engine_req, &volume[..], shape))
            .await
            .map_err(|e| anyhow!("onnx join: {e}"))?
            .map_err(|e| anyhow!("onnx inference: {e}"))?;
        log::info!("[AI/ONNX] model={} inference finished", req.model);
        Ok(result)
    }
}