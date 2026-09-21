//! ONNX-based segmentation engine using `ort` 2.0-rc.x.
//!
//! Pipeline for TotalSegmentator-exported ONNX:
//! 1. preprocess: shape `[1, 1, D, H, W]`, HU -> normalized float
//! 2. run_inference: single-input/single-output model
//! 3. postprocess: argmax over the 9 output channels -> `[D, H, W]` u8
//!
//! Output byte order matches `RenderContent::from_labels_r8` (Z, Y, X
//! row-major). `0 = background`, 1..7 = L1..sacrum (matching
//! `LABEL_TABLE` in `src/server/python/inference.py`).
//!
//! The exported model (see `scripts/export_totalsegmentator_onnx.py`) has
//! exactly 9 output channels whose argmax is byte-identical to the full
//! 27-class nnU-Net argmax:
//!   channel 0  = background
//!   channel 1..7 = L1, L2, L3, L4, L5, S1, sacrum  (kepler label id == channel)
//!   channel 8  = "rest" = max over every other TS class (collapsed)
//! The host writes `0` when the winner is channel 0 (background) or the
//! last channel (rest); otherwise it writes the channel index, which is the
//! kepler label id. So the ONNX and Python backends produce the same mask.

//! Note on the ort API: this module assumes `ort` 2.0-rc.4 / rc.13.
//! `Outlet` field accesses are private in this version, so shape / name
//! are read through accessor methods (`Outlet::name`, `Outlet::dtype`).

use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{anyhow, Result};
use ort::{
    ep,
    session::Session,
    session::builder::{GraphOptimizationLevel, SessionBuilder},
    value::{Tensor, ValueType},
};
use parking_lot::Mutex;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock as AsyncRwLock;

/// Engine-level mirror of `ai_model::SegmentRequest`. Adds `fast_mode`,
/// which is forwarded to TotalSegmentator's `fast` flag and also used
/// by the ONNX path to pick the right input spacing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SegmentRequest {
    pub series_id: String,
    pub model: String,
    #[serde(default = "default_fast_mode")]
    pub fast_mode: bool,
}

fn default_fast_mode() -> bool { true }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressResponse {
    pub task_id: String,
    pub status: String,
    pub progress: u8,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SegmentResult {
    pub task_id: String,
    pub volume: String,
    pub labels: std::collections::HashMap<String, String>,
    /// `(width, height, depth)` in `(X, Y, Z)` voxel order.
    pub dimensions: (u32, u32, u32),
    pub mask_data: Vec<u8>,
    /// Blended network logits on the resampled (1.5 mm) grid, laid out
    /// `[C, D, H, W]` row-major and already divided by the Gaussian weight
    /// sum. Populated by the ONNX backend only; the Python backend leaves it
    /// empty (it returns a finished mask).
    ///
    /// The caller needs these to project the mask back onto the original
    /// voxel grid the way nnU-Net does — resample the *logits* and argmax
    /// there. Upsampling the argmaxed labels with nearest neighbour instead
    /// costs ~0.94 Dice on real data; see
    /// `crate::server::resample::upsample_logits_argmax`.
    #[serde(skip)]
    pub logits: Vec<f32>,
    /// Channel count of `logits` (0 when `logits` is empty).
    #[serde(skip)]
    pub num_classes: usize,
}

#[derive(Debug, Clone)]
struct ModelInfo {
    input_name: String,
    output_name: String,
    #[allow(dead_code)]
    input_shape: Vec<i64>,
    #[allow(dead_code)]
    output_shape: Vec<i64>,
    /// Number of channels we actually use for argmax. Always
    /// `min(total_classes, label_table.len())`.
    num_classes_used: usize,
    label_table: Vec<String>,
    #[allow(dead_code)]
    spacing_mm: [f32; 3],
    /// nnU-Net `CTNormalization` parameters for this model. The network was
    /// trained on data preprocessed with: `x = clip(x, clip_low, clip_high);
    /// x = (x - mean) / std`. These values are the foreground statistics of
    /// Dataset292 (TotalSegmentator vertebrae, 3d_fullres) read from
    /// `plans_manager.foreground_intensity_properties_per_channel["0"]`.
    norm_clip_low: f32,
    norm_clip_high: f32,
    norm_mean: f32,
    norm_std: f32,
}

pub struct OnnxSegmentator {
    #[allow(dead_code)]
    model_path: String,
    /// `Session::run` requires `&mut self` in ort 2.0, so the session
    /// stays behind a `Mutex`. The sliding-window patch loop still
    /// parallelises CPU work (patch extraction + nnU-Net normalisation)
    /// across rayon workers; the GPU forward serialises on this lock but
    /// benefits from DirectML (GPU EP) + Level3 graph optimisation.
    session: Arc<Mutex<Session>>,
    model_info: ModelInfo,
}

impl OnnxSegmentator {
    pub fn new(model_path: &str) -> Result<Self> {
        log::info!("[AI/ONNX] Loading model from: {}", model_path);

        if !Path::new(model_path).exists() {
            return Err(anyhow!("Model file not found: {}", model_path));
        }

        // Try DirectML (GPU) first; fall back to CPU-only if the DirectML
        // runtime is missing or no DX12 GPU is available. The fallback keeps
        // kepler.exe functional on machines without a compatible GPU.
        let session = {
            let try_gpu = || -> Result<Session> {
                SessionBuilder::new()
                    .map_err(|e| anyhow!("session builder: {e}"))?
                    .with_optimization_level(GraphOptimizationLevel::Level3)
                    .map_err(|e| anyhow!("optimization level: {e}"))?
                    .with_execution_providers([ep::DirectML::default().build()])
                    .map_err(|e| anyhow!("directml ep: {e}"))?
                    .commit_from_file(model_path)
                    .map_err(|e| anyhow!("commit_from_file: {e}"))
            };
            match try_gpu() {
                Ok(s) => {
                    log::info!("[AI/ONNX] DirectML EP enabled for {}", model_path);
                    s
                }
                Err(e) => {
                    log::warn!(
                        "[AI/ONNX] DirectML unavailable ({e}); falling back to CPU-only session"
                    );
                    SessionBuilder::new()
                        .map_err(|e| anyhow!("session builder: {e}"))?
                        .with_optimization_level(GraphOptimizationLevel::Level3)
                        .map_err(|e| anyhow!("optimization level: {e}"))?
                        .commit_from_file(model_path)
                        .map_err(|e| anyhow!("commit_from_file (CPU fallback): {e}"))?
                }
            }
        };

        let model_info = Self::extract_model_info(&session, model_path)?;

        Ok(Self {
            model_path: model_path.to_string(),
            session: Arc::new(Mutex::new(session)),
            model_info,
        })
    }

    fn extract_model_info(session: &Session, model_path: &str) -> Result<ModelInfo> {
        let inputs = session.inputs();
        let outputs = session.outputs();
        anyhow::ensure!(inputs.len() == 1, "model must have exactly 1 input, got {}", inputs.len());
        anyhow::ensure!(outputs.len() == 1, "model must have exactly 1 output, got {}", outputs.len());

        let input = &inputs[0];
        let output = &outputs[0];

        let input_shape = tensor_shape(input.dtype(), "input")?;
        let output_shape = tensor_shape(output.dtype(), "output")?;

        let total_classes = output_shape.get(1).copied().unwrap_or(0).max(0) as usize;
        let label_table = spine_label_table();
        // Use every output channel the model actually emits. The 9-channel
        // export keeps background (ch 0), the 7 spine classes (ch 1..7) and a
        // collapsed "rest" (last channel); argmax runs over all of them so the
        // background/non-spine voxels correctly resolve to label 0. (The
        // 7-entry `label_table` is only the human-readable spine names for the
        // `labels` map, NOT a cap on the argmax channel count.)
        let num_classes_used = if total_classes == 0 {
            label_table.len()
        } else {
            total_classes
        };

        let spacing_mm = if model_path.to_ascii_lowercase().contains("_fast") {
            [1.5, 1.5, 1.5]
        } else {
            [1.5, 1.5, 1.5]
        };

        // Dataset292 (TotalSegmentator vertebrae, 3d_fullres) nnU-Net
        // CTNormalization foreground statistics. These are NOT the coarse
        // `(x-1000)/400` placeholder; they are the exact clip bounds and
        // z-score params the trained network expects, so the ONNX path must
        // replicate them to match the Python TotalSegmentator output.
        let (norm_clip_low, norm_clip_high, norm_mean, norm_std) =
            (-96.0_f32, 1514.0_f32, 367.3293_f32, 320.8587_f32);

        Ok(ModelInfo {
            input_name: input.name().to_string(),
            output_name: output.name().to_string(),
            input_shape,
            output_shape,
            num_classes_used,
            label_table,
            spacing_mm,
            norm_clip_low,
            norm_clip_high,
            norm_mean,
            norm_std,
        })
    }

    /// Run segmentation on a single-volume float buffer (HU values) laid
    /// out in **Z, Y, X** row-major order, with the given
    /// `(depth, height, width)` spatial dims. Returns the multi-label mask
    /// in Z, Y, X row-major order so the layout matches
    /// `RenderContent::from_labels_r8`.
    pub fn segment(
        &self,
        req: SegmentRequest,
        volume_data: &[f32],
        volume_shape: (usize, usize, usize), // (depth, height, width)
    ) -> Result<SegmentResult> {
        let task_id = uuid::Uuid::new_v4().to_string();
        let (depth, height, width) = volume_shape;
        log::info!(
            "Starting ONNX segmentation for task {} (model={}) volume={:?}",
            task_id, req.model, volume_shape
        );

        // Pad each spatial axis to a multiple of 64. The nnU-Net backbone
        // has total stride 32 (5x stride-2). If any intermediate dim is odd,
        // the encoder/decoder skip-connection concat mismatches at runtime
        // (ort error: "Axis 2 has mismatched dimensions of 15 and 16").
        // Padding to a multiple of 64 keeps every downsample level even.
        let (padded, front, (pd, ph, pw)) = pad_volume_to_multiple(volume_data, volume_shape, 64)?;

        // Sliding-window inference. Running the full padded volume
        // (256x512x512) in a single forward pass needs ~17GB for one
        // full-res activation tensor and OOMs, so we tile it into fixed
        // patches with 50% overlap and blend predictions with a Gaussian
        // weight (mirroring nnU-Net's default sliding-window scheme).
        let patch = (
            PATCH_DEPTH.min(pd),
            PATCH_HEIGHT.min(ph),
            PATCH_WIDTH.min(pw),
        );
        // nnU-Net sliding window: `tile_step_size = 0.5` (50% overlap). The
        // starts are spread *evenly* so the last patch ends exactly at
        // `dim - tile` (see `compute_steps_for_sliding_window`); a fixed
        // stride would only coincide when `(dim - tile)` is divisible by
        // `tile * step_size`.

        // Probe one patch to learn the output channel count.
        let probe = self.infer_patch(&padded[..patch.0 * patch.1 * patch.2], patch)?;
        // Use the model's real channel count (9 for the spine export).
        // Do NOT cap at `label_table.len()` (7): that would drop the
        // background (ch 0) and "rest" (last ch) channels and force every
        // voxel to a foreground label.
        let num_classes = (probe.0[1] as usize)
            .min(self.model_info.num_classes_used)
            .max(1);

        let gauss = gaussian_weight(patch);
        let mut acc = vec![0.0f32; num_classes * pd * ph * pw];
        let mut wsum = vec![0.0f32; pd * ph * pw];

        // Collect all patch start coordinates.
        let coords: Vec<(usize, usize, usize)> = patch_starts(pd, patch.0, TILE_STEP_SIZE)
            .into_iter()
            .flat_map(|z0| {
                patch_starts(ph, patch.1, TILE_STEP_SIZE)
                    .into_iter()
                    .flat_map(move |y0| {
                        patch_starts(pw, patch.2, TILE_STEP_SIZE)
                            .into_iter()
                            .map(move |x0| (z0, y0, x0))
                    })
            })
            .collect();

        let total = coords.len();
        log::info!(
            "[AI/ONNX] sliding-window: {} patches of {:?} (step {}) on padded {:?}",
            total, patch, TILE_STEP_SIZE, (pd, ph, pw)
        );

        // Parallel inference: each patch is independently forwarded. The
        // `ort::Session` is `Send + Sync`, so rayon can call `infer_patch`
        // from multiple workers concurrently, letting the GPU overlap
        // forward passes across patches. Patch extraction + nnU-Net
        // normalization happen on each worker; the GPU forward overlaps.
        let results: Vec<(usize, usize, usize, Vec<f32>)> = coords
            .par_iter()
            .enumerate()
            .filter_map(|(i, &(z0, y0, x0))| {
                let mut buf = vec![0.0f32; patch.0 * patch.1 * patch.2];
                for z in 0..patch.0 {
                    let pz = z0 + z;
                    for y in 0..patch.1 {
                        let py = y0 + y;
                        for x in 0..patch.2 {
                            buf[(z * patch.1 + y) * patch.2 + x] =
                                padded[(pz * ph + py) * pw + (x0 + x)];
                        }
                    }
                }
                match self.infer_patch(&buf, patch) {
                    Ok((_oshape, out)) => {
                        if i > 0 && i % 50 == 0 {
                            log::info!("[AI/ONNX] patch {}/{}", i, total);
                        }
                        Some((z0, y0, x0, out))
                    }
                    Err(e) => {
                        log::error!(
                            "[AI/ONNX] patch ({},{},{}) inference failed: {}",
                            z0, y0, x0, e
                        );
                        None
                    }
                }
            })
            .collect();

        // Serial Gaussian-weighted accumulation. Pure elementwise FMA —
        // cheap relative to the network forward, so serial is fine.
        for (z0, y0, x0, out) in &results {
            // out layout: [N=1, C, Z, Y, X]
            for c in 0..num_classes {
                for z in 0..patch.0 {
                    let oz = z0 + z;
                    for y in 0..patch.1 {
                        let oy = y0 + y;
                        for x in 0..patch.2 {
                            let ox = x0 + x;
                            let w = gauss[(z * patch.1 + y) * patch.2 + x];
                            let oi = ((c * patch.0 + z) * patch.1 + y) * patch.2 + x;
                            acc[(c * pd + oz) * ph * pw + oy * pw + ox] += out[oi] * w;
                            if c == 0 {
                                wsum[oz * ph * pw + oy * pw + ox] += w;
                            }
                        }
                    }
                }
            }
        }

        // Normalize the blended logits and argmax back into the original extent.
        let (fd, fh, fw) = front;
        anyhow::ensure!(
            fd + depth <= pd && fh + height <= ph && fw + width <= pw,
            "padded volume {}x{}x{} too small for original {}x{}x{} + front {},{},{}",
            pd, ph, pw, depth, height, width, fd, fh, fw
        );

        // Compact the blended, weight-normalised logits into an unpadded
        // `[C, D, H, W]` buffer. nnU-Net resamples THESE (not the argmaxed
        // labels) back onto the original grid, so the caller needs them to
        // reproduce the reference mask.
        let dhw = height * width;
        let mut logits = vec![0.0f32; num_classes * depth * dhw];
        for c in 0..num_classes {
            for z in 0..depth {
                let oz = fd + z;
                for y in 0..height {
                    let oy = fh + y;
                    for x in 0..width {
                        let ox = fw + x;
                        let w = wsum[oz * ph * pw + oy * pw + ox];
                        let mut v = acc[(c * pd + oz) * ph * pw + oy * pw + ox];
                        if w > 0.0 {
                            v /= w;
                        }
                        logits[(c * depth + z) * dhw + y * width + x] = v;
                    }
                }
            }
        }

        // Coarse argmax on the resampled grid. Kept as the fallback label
        // map (and it is tiny next to the logits buffer). ONNX output layout
        // (see export script):
        //   ch 0    = background
        //   ch 1..7 = L1..sacrum (kepler label id == channel index)
        //   ch last = "rest" (max over all other TS classes)
        // Both background and rest collapse to label 0; otherwise the channel
        // index is the kepler label id, so the ONNX and Python backends emit
        // the same mask.
        let mut mask = vec![0u8; depth * dhw];
        for i in 0..mask.len() {
            let mut best_c = 0usize;
            let mut best_v = f32::MIN;
            for c in 0..num_classes {
                let v = logits[c * dhw + i];
                if v > best_v {
                    best_v = v;
                    best_c = c;
                }
            }
            mask[i] = if best_c == 0 || best_c == num_classes - 1 {
                0u8
            } else {
                best_c as u8
            };
        }

        let mut labels = std::collections::HashMap::new();
        for (i, name) in self.model_info.label_table.iter().enumerate() {
            labels.insert((i + 1).to_string(), name.clone());
        }

        Ok(SegmentResult {
            task_id: task_id.clone(),
            volume: format!("segment_{}", task_id),
            labels,
            dimensions: (width as u32, height as u32, depth as u32),
            mask_data: mask,
            logits,
            num_classes,
        })
    }

    /// Build an `f32` tensor with shape `[1,1,D,H,W]` from `volume_data`
    /// (Z,Y,X row-major) and the *runtime* spatial dims. The ONNX model
    /// uses dynamic spatial dims, so we derive them from the actual volume
    /// rather than the (possibly `-1`) static input shape.
    fn build_input_tensor(
        &self,
        volume_data: &[f32],
        (depth, height, width): (usize, usize, usize),
    ) -> Result<Tensor<f32>> {
        let shape: Vec<i64> = vec![1, 1, depth as i64, height as i64, width as i64];

        // channels == 1, so the expected element count is depth*height*width
        let expected = depth * height * width;
        anyhow::ensure!(
            volume_data.len() == expected,
            "volume_data length {} != expected {} (D,H,W={:?})",
            volume_data.len(), expected, (depth, height, width)
        );

        // nnU-Net CTNormalization, matching the Python TotalSegmentator
        // (Dataset292 vertebrae) preprocessing exactly:
        //   x = clip(x, clip_low, clip_high); x = (x - mean) / std
        // Applied per-voxel on raw HU before the network forward. Spacing
        // resampling to the model's 1.5 mm target is done upstream in
        // `ai_handler::run_segmentation` (see `resample::resample_volume_cubic`)
        // before this function is ever called.
        let norm = &self.model_info;
        let normalized: Vec<f32> = volume_data
            .iter()
            .map(|&x| {
                let c = x.clamp(norm.norm_clip_low, norm.norm_clip_high);
                (c - norm.norm_mean) / norm.norm_std
            })
            .collect();

        let tensor = Tensor::from_array((shape, normalized))?;
        Ok(tensor)
    }

    fn run_inference(&self, input_tensor: Tensor<f32>) -> Result<(Vec<i64>, Vec<f32>)> {
        // ort 2.0: `Session::run` takes `&mut self`, so the session is
        // behind a `Mutex`. The GPU forward serialises here, but the
        // CPU-bound patch extraction + normalisation in the rayon loop
        // overlaps across workers.
        let mut session = self.session.lock();
        let outputs = session.run(ort::inputs![self.model_info.input_name.as_str() => input_tensor])?;

        let output_value = outputs
            .get(self.model_info.output_name.as_str())
            .ok_or_else(|| anyhow!("output `{}` not produced by session", self.model_info.output_name))?;

        let (shape, data) = output_value.try_extract_tensor::<f32>()?;
        Ok((shape.to_vec(), data.to_vec()))
    }

    /// Run a single forward pass on a `[1,1,D,H,W]` patch and return the raw
    /// `(shape, data)` where `data` is laid out `[N, C, Z, Y, X]`.
    fn infer_patch(
        &self,
        patch_data: &[f32],
        (pd, ph, pw): (usize, usize, usize),
    ) -> Result<(Vec<i64>, Vec<f32>)> {
        let tensor = self.build_input_tensor(patch_data, (pd, ph, pw))?;
        self.run_inference(tensor)
    }

}

/// Extract the `i64` shape from an `Outlet`'s `ValueType`.
/// Dynamic dims (`-1`) are **allowed**: the model is fully convolutional
/// and accepts arbitrary spatial sizes, so we resolve the real dims at
/// inference time from the actual input volume rather than the (possibly
/// `-1`) static shape. We only require the tensor to be 5D `[N,C,D,H,W]`.
fn tensor_shape(dtype: &ValueType, kind: &str) -> Result<Vec<i64>> {
    match dtype {
        ValueType::Tensor { shape, .. } => {
            let dims: Vec<i64> = shape.iter().copied().collect();
            anyhow::ensure!(
                dims.len() == 5,
                "{kind} tensor must be 5D [N,C,D,H,W], got {dims:?}"
            );
            Ok(dims)
        }
        other => Err(anyhow!("expected {kind} tensor, got {other:?}")),
    }
}

/// Compact 6-spine + sacrum label table aligned with the Python backend
/// (`src/server/python/inference.py`). Channel order must match the ONNX
/// export: `[L1, L2, L3, L4, L5, S1, sacrum]`.
fn spine_label_table() -> Vec<String> {
    vec![
        "L1".to_string(),
        "L2".to_string(),
        "L3".to_string(),
        "L4".to_string(),
        "L5".to_string(),
        "S1".to_string(),
        "sacrum".to_string(),
    ]
}

/// Pad a Z,Y,X row-major float volume to the next multiple of `m` along
/// each spatial axis, filling the border with **constant 0** so it matches
/// nnU-Net's `pad_nd_image(..., mode='constant', constant_values=0)`. The
/// original region is kept centered (mirroring nnU-Net's even padding) and is
/// reachable at `[front, front + original)` inside the padded tensor; only the
/// padded margin is zero, which the sliding window discards on read-back.
fn pad_volume_to_multiple(
    data: &[f32],
    (d, h, w): (usize, usize, usize),
    m: usize,
) -> Result<(Vec<f32>, (usize, usize, usize), (usize, usize, usize))> {
    anyhow::ensure!(
        data.len() == d * h * w,
        "volume_data length {} != expected {} (D,H,W={:?})",
        data.len(), d * h * w, (d, h, w)
    );

    let pd = ((d + m - 1) / m) * m;
    let ph = ((h + m - 1) / m) * m;
    let pw = ((w + m - 1) / m) * m;
    let fd = (pd - d) / 2;
    let fh = (ph - h) / 2;
    let fw = (pw - w) / 2;

    let mut out = vec![0.0f32; pd * ph * pw];
    for zc in 0..pd {
        // Source index in the original volume along Z; out-of-range (padded
        // margin) voxels stay 0.
        let zo = zc as isize - fd as isize;
        for yc in 0..ph {
            let yo = yc as isize - fh as isize;
            for xc in 0..pw {
                let xo = xc as isize - fw as isize;
                if zo >= 0 && zo < d as isize && yo >= 0 && yo < h as isize && xo >= 0 && xo < w as isize {
                    out[(zc * ph + yc) * pw + xc] =
                        data[(zo as usize * h + yo as usize) * w + xo as usize];
                }
            }
        }
    }
    Ok((out, (fd, fh, fw), (pd, ph, pw)))
}

/// Sliding-window patch size (Z, Y, X). Matches the ONNX export's training
/// patch; small enough that a single patch's activations fit in RAM while
/// still capturing full-resolution context. Must divide the padded dims
/// (which are multiples of 64) for clean tiling.
const PATCH_DEPTH: usize = 128;
const PATCH_HEIGHT: usize = 128;
const PATCH_WIDTH: usize = 128;

/// Sliding-window tile step size (fraction of the patch). nnU-Net uses
/// `tile_step_size = 0.5` for `tile_step_size` in `predict_from_raw_data`.
const TILE_STEP_SIZE: f64 = 0.5;

/// List of patch start offsets along one axis so the union of
/// `[start, start+patch)` covers `[0, dim)`. Mirrors nnU-Net's
/// `compute_steps_for_sliding_window`:
/// ```text
/// target_step = patch * tile_step_size
/// num_steps   = ceil((dim - patch) / target_step) + 1
/// actual_step = (dim - patch) / (num_steps - 1)   # evenly spaced
/// starts      = [round(actual_step * i) for i in range(num_steps)]
/// ```
/// The last start is exactly `dim - patch`, so the patches tile the whole
/// axis with even overlap (not merely a fixed stride + tail).
fn patch_starts(dim: usize, patch: usize, tile_step_size: f64) -> Vec<usize> {
    if patch >= dim {
        return vec![0];
    }
    let target_step = patch as f64 * tile_step_size;
    let num_steps = (((dim as f64 - patch as f64) / target_step).ceil() as i64)
        .max(1) as usize
        + 1;
    let max_step_value = dim - patch; // last start
    let actual_step = if num_steps > 1 {
        max_step_value as f64 / (num_steps - 1) as f64
    } else {
        0.0
    };
    let mut starts = Vec::with_capacity(num_steps);
    for i in 0..num_steps {
        starts.push((actual_step * i as f64).round() as usize);
    }
    starts
}

/// 3D Gaussian weight of shape `[pd, ph, pw]` centered in the patch, used to
/// blend overlapping sliding-window predictions (high weight near the patch
/// center, ~0 at the edges). Mirrors nnU-Net's `sigma = patch_size / 8`.
fn gaussian_weight((pd, ph, pw): (usize, usize, usize)) -> Vec<f32> {
    let sigma = |s: usize| {
        let v = (s as f32) / 8.0;
        if v < 1e-4 {
            1e-4
        } else {
            v
        }
    };
    let sz = sigma(pd);
    let sy = sigma(ph);
    let sw = sigma(pw);
    // nnU-Net `compute_gaussian` centers the importance map at
    // `center_coords = [i // 2 for i in tile_size]` (integer floor division,
    // so 128 -> 64.0, not 63.5). Match that exactly.
    let cz = (pd / 2) as f32;
    let cy = (ph / 2) as f32;
    let cx = (pw / 2) as f32;
    let mut g = vec![0.0f32; pd * ph * pw];
    for z in 0..pd {
        let dz = (z as f32 - cz) / sz;
        for y in 0..ph {
            let dy = (y as f32 - cy) / sy;
            for x in 0..pw {
                let dx = (x as f32 - cx) / sw;
                g[(z * ph + y) * pw + x] = (-0.5 * (dz * dz + dy * dy + dx * dx)).exp();
            }
        }
    }
    g
}

pub struct ModelManager {
    models: Arc<AsyncRwLock<std::collections::HashMap<String, Arc<OnnxSegmentator>>>>,
    model_dir: PathBuf,
}

impl ModelManager {
    pub async fn new(model_dir: &str) -> Result<Self> {
        std::fs::create_dir_all(model_dir)?;
        Ok(Self {
            models: Arc::new(AsyncRwLock::new(std::collections::HashMap::new())),
            model_dir: PathBuf::from(model_dir),
        })
    }

    pub fn model_dir(&self) -> &Path { &self.model_dir }

    /// Cheap probe used by the handler to dispatch ONNX-direct vs
    /// Python-proxy without actually loading the session.
    pub fn has_model(&self, model_name: &str) -> bool {
        self.model_dir.join(format!("{}.onnx", model_name)).exists()
    }

    pub async fn get_model(&self, model_name: &str) -> Result<Arc<OnnxSegmentator>> {
        {
            let models = self.models.read().await;
            if let Some(m) = models.get(model_name) { return Ok(m.clone()); }
        }
        let path = self.model_dir.join(format!("{}.onnx", model_name));
        let segmentator = Arc::new(OnnxSegmentator::new(path.to_str().unwrap())?);
        let mut models = self.models.write().await;
        models.insert(model_name.to_string(), segmentator);
        Ok(models.get(model_name).unwrap().clone())
    }

    /// Pre-warm the registry with the given model names. Names that do
    /// not have a corresponding `.onnx` on disk are skipped (warning).
    pub async fn preload(&self, model_names: &[&str]) -> Result<()> {
        for name in model_names {
            if self.model_dir.join(format!("{}.onnx", name)).exists() {
                self.get_model(name).await?;
            } else {
                log::warn!("Skipping preload: {}.onnx not found in {}", name, self.model_dir.display());
            }
        }
        Ok(())
    }

    /// Pre-warm every `.onnx` file in the model directory.
    pub async fn preload_all(&self) -> Result<()> {
        let mut entries = tokio::fs::read_dir(&self.model_dir).await?;
        let mut names = Vec::new();
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("onnx") {
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                    names.push(stem.to_string());
                }
            }
        }
        let refs: Vec<&str> = names.iter().map(|s| s.as_str()).collect();
        self.preload(&refs).await
    }
}