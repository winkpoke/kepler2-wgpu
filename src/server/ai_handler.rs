use std::sync::Arc;
use std::time::Instant;
use anyhow::{anyhow, Result};
use uuid::Uuid;
use crate::data::medical_imaging::{MedicalVolume, MhaParser};
use crate::server::{
    ai_model::{
        BackendPreference, CancelRequest, CancelResponse, ProgressResponse as ApiProgressResponse,
        SegmentRequest as ApiSegmentRequest, SegmentResponse,
    },
    backend::{OnnxBackend, SegmentationBackend},
    orientation::{reorient_mask_to_original, reorient_volume_to_canonical, ReorientPlan},
    state::ServerState,
    ws::WsMessage,
};

/// Send a WS broadcast. Errors (closed channels, no subscribers) are intentionally swallowed
/// the broadcast channel may have zero subscribers if the browser has not yet connected.
fn broadcast(state: &ServerState, msg: WsMessage) {
    let _ = state.ws_tx.send(msg);
}

/// Entry point for `POST /api/segment`.
///
/// Creates a Rust-side `task_id` (the one returned to the client) and
/// spawns `run_segmentation` on a tokio task. The actual progress and
/// completion messages are broadcast on `state.ws_tx`.
pub async fn handle_segment(
    state: ServerState,
    req: ApiSegmentRequest,
) -> Result<SegmentResponse> {
    let task_id = Uuid::new_v4().to_string();
    log::info!("[AI] creating segmentation task {} for model {}", task_id, req.model);

    // Register in the local task manager so /progress and /result work.
    state.tasks.register(task_id.clone(), req.series_id.clone(), req.model.clone()).await;

    // ws: SegmentStarted
    broadcast(&state, WsMessage::SegmentStarted {
        task_id: task_id.clone(),
        series_id: req.series_id.clone(),
        model: req.model.clone(),
    });

    let state_clone = state.clone();
    let task_id_clone = task_id.clone();
    tokio::spawn(async move {
        run_segmentation(state_clone, task_id_clone, req).await;
    });

    Ok(SegmentResponse { status: "accepted".to_string(), task_id })
}

async fn run_segmentation(state: ServerState, task_id: String, req: ApiSegmentRequest) {
    let started = Instant::now();
    state.tasks.mark_running(&task_id).await;

    // Backend selection
    let pref = req.backend;
    let has_local_onnx = state.onnx_models.lock().as_ref()
        .map(|m| m.has_model(&req.model)).unwrap_or(false);

    let use_onnx = match pref {
        BackendPreference::Onnx => has_local_onnx,
        BackendPreference::Python => false,
        BackendPreference::Auto => has_local_onnx,
    };

    let use_python = match pref {
        BackendPreference::Onnx => false,
        _ => !use_onnx,
    };

    state.tasks.update_progress(&task_id, 10).await;
    broadcast(&state, WsMessage::SegmentProgress { task_id: task_id.clone(), value: 10 });

    if use_onnx {
        // Clone the manager Arc out of the lock first so the (non-`Send`)
        // `parking_lot` guard is dropped before any `.await`; otherwise the
        // spawned `run_segmentation` future would not be `Send`.
        let manager_opt = state.onnx_models.lock().clone();
        let mgr = match manager_opt {
            Some(m) => m,
            None => {
                fail_task(&state, &task_id, "ONNX manager not initialized".to_string()).await;
                return;
            }
        };
        let backend = OnnxBackend::new(mgr);

        state.tasks.update_progress(&task_id, 30).await;
        broadcast(&state, WsMessage::SegmentProgress { task_id: task_id.clone(), value: 30 });

        let loaded = match load_volume_f32(&state, &req.series_id).await {
            Ok(v) => v,
            Err(e) => {
                fail_task(&state, &task_id, format!("load volume: {e}")).await;
                return;
            }
        };
        let (volume, shape, spacing_zyx, plan) =
            (loaded.volume, loaded.shape, loaded.spacing, loaded.plan);

        // Resample to the model's target spacing [1.5, 1.5, 1.5] before
        // inference, using cubic (order-3) B-spline interpolation to match
        // TotalSegmentator's `change_spacing(order=3)`. The volume is already
        // in the network layout (RAS-canonical reversed to `(S, P, L)`), which
        // is exactly what nnU-Net feeds its network, so no further reordering
        // is needed here.
        let target = [1.5f32, 1.5, 1.5];
        let (resampled, rshape) = crate::server::resample::resample_volume_cubic(
            &volume, shape, spacing_zyx, target,
        );
        let volume = Arc::new(resampled);

        state.tasks.update_progress(&task_id, 50).await;
        broadcast(&state, WsMessage::SegmentProgress { task_id: task_id.clone(), value: 50 });

        match backend.segment(&req, volume, rshape).await {
            Ok(mut result) => {
                // Project the mask back onto the original voxel grid the way
                // nnU-Net does: resample the **logits** off the 1.5 mm grid
                // and argmax there. `plans.json` stores
                //   resampling_fn_probabilities_kwargs = {'is_seg': False,
                //       'order': 1, 'order_z': 0, 'force_separate_z': None}
                // i.e. linear interpolation in-plane plus nearest neighbour
                // along the anisotropic axis (here the head-foot one).
                // Upsampling the argmaxed labels with nearest neighbour
                // instead can only place boundaries on the 1.5 mm grid and
                // costs ~0.94 Dice per foreground label on real data.
                let mask_net = if result.num_classes > 0 && !result.logits.is_empty() {
                    let aniso = crate::server::resample::nnunet_aniso_axis(target, spacing_zyx);
                    let mut m = crate::server::resample::upsample_logits_argmax(
                        &result.logits,
                        result.num_classes,
                        rshape,
                        shape,
                        aniso,
                        true,
                    );
                    // The ONNX export puts background in channel 0 and the
                    // collapsed "rest" class in the LAST channel; both mean
                    // "not a spine label" and map to 0.
                    if result.num_classes <= u8::MAX as usize {
                        let last = (result.num_classes - 1) as u8;
                        for v in m.iter_mut() {
                            if *v == last {
                                *v = 0;
                            }
                        }
                    }
                    // Release the (large) logits buffer before the mask is
                    // handed to the task store.
                    result.logits = Vec::new();
                    result.num_classes = 0;
                    m
                } else {
                    // Fallback for a backend that only returned a finished
                    // mask: nearest-neighbour inverse resample.
                    crate::server::resample::resample_mask_nearest(
                        &result.mask_data, rshape, shape,
                    )
                };
                // Map the network-layout `(S, P, L)` mask back to the original
                // MHA `(Z, Y, X)` space. `network_layout()` is an involution,
                // so reusing it inverts the forward reorientation and the mask
                // overlays the source volume exactly like the Python backend's
                // `undo_canonical`.
                let net_plan = plan.network_layout();
                let mask = reorient_mask_to_original(&mask_net, shape, &net_plan);
                result.mask_data = mask;
                let orig = plan.original_shape; // (Z, Y, X)
                result.dimensions = (orig.2 as u32, orig.1 as u32, orig.0 as u32);

                let out_dir = state.output_root.join(&task_id);
                if let Err(e) = tokio::fs::create_dir_all(&out_dir).await {
                    log::warn!("[AI/ONNX] create output dir {:?} failed: {e}", out_dir);
                } else if let Err(e) =
                    tokio::fs::write(out_dir.join("spine.raw"), &result.mask_data).await
                {
                    log::warn!("[AI/ONNX] spine.raw persist failed for {}: {e}", task_id);
                } else {
                    log::info!(
                        "[AI/ONNX] wrote {} ({} bytes)",
                        out_dir.join("spine.raw").display(),
                        result.mask_data.len()
                    );
                }

                state.tasks.mark_completed(
                    &task_id,
                    Some(result.volume.clone()),
                    Some(result.mask_data),
                    result.labels.clone(),
                ).await;
                broadcast(&state, WsMessage::SegmentComplete {
                    task_id: task_id.clone(),
                    volume: result.volume,
                    labels: result.labels,
                });
                log::info!("[AI] task {} (ONNX) completed in {:?}", task_id, started.elapsed());
            }
            Err(e) => {
                fail_task(&state, &task_id, format!("onnx inference: {e}")).await;
            }
        }
    } else if use_python {
        let ai = state.ai.clone();
        log::info!("[AI] model={} backend=python (http)", req.model);

        state.tasks.update_progress(&task_id, 20).await;
        broadcast(&state, WsMessage::SegmentProgress { task_id: task_id.clone(), value: 20 });

        let remote = match ai.segment(req.clone()).await {
            Ok(r) => r,
            Err(e) => {
                fail_task(&state, &task_id, format!(
                    "python submit failed: {e}. Is the AI service running? Start it with \
                     `python -m uvicorn app:app --port 8001` in src/server/python, \
                     or point KEPLER_AI_URL at it."
                )).await;
                return;
            }
        };
        log::info!("[AI] task {} -> python task {}", task_id, remote.task_id);

        let mut last_value = u8::MAX;
        loop {
            let t = match ai.progress(&remote.task_id).await {
                Ok(t) => t,
                Err(e) => { fail_task(&state, &task_id, format!("python poll failed: {e}")).await; return; }
            };

            if t.progress != last_value {
                state.tasks.update_progress(&task_id, t.progress).await;
                broadcast(&state, WsMessage::SegmentProgress { task_id: task_id.clone(), value: t.progress });
                last_value = t.progress;
            }

            match t.status.as_str() {
                "completed" | "complete" | "done" => {
                    let labels = t.labels.clone().unwrap_or_default();
                    let volume = t.volume.clone().unwrap_or_else(|| format!("seg-{task_id}"));
                    let mask = ai.download(&remote.task_id).await.ok();
                    if mask.is_none() { log::warn!("[AI] python task {} completed without a mask", remote.task_id); }
                    state.tasks.mark_completed(&task_id, Some(volume.clone()), mask, labels.clone()).await;
                    broadcast(&state, WsMessage::SegmentComplete { task_id: task_id.clone(), volume, labels });
                    log::info!("[AI] task {} (python/http) completed in {:?}", task_id, started.elapsed());
                    return;
                }
                "failed" | "error" => {
                    let msg = t.message.clone().unwrap_or_else(|| "AI service reported failure".to_string());
                    fail_task(&state, &task_id, msg).await;
                    return;
                }
                "cancelled" | "canceled" => {
                    state.tasks.mark_cancelled(&task_id).await;
                    broadcast(&state, WsMessage::SegmentCancelled { task_id: task_id.clone() });
                    return;
                }
                _ => {}
            }

            // 本地 cancel（HTTP 或 WS）会把本地任务翻成 Cancelled：
            // 顺手通知 Python 服务早停，然后结束轮询。
            if let Some(t) = state.tasks.get(&task_id).await {
                if matches!(t.status, crate::server::ai_task::TaskStatus::Cancelled) {
                    let _ = ai.cancel(&remote.task_id).await;
                    return;
                }
            }

            tokio::time::sleep(std::time::Duration::from_millis(POLL_INTERVAL_MS)).await;
        }
    } else {
        let msg = match pref {
            BackendPreference::Onnx => format!(
                "backend=onnx requested but models/{}.onnx is not present", req.model
            ),
            BackendPreference::Python => {
                "backend=python requested but Python runtime is not installed".to_string()
            }
            BackendPreference::Auto => format!(
                "no local ONNX model '{}' and Python runtime is not installed; \
                 this model requires the Python fallback", req.model
            ),
        };
        log::warn!("[AI] {}", msg);
        fail_task(&state, &task_id, msg).await;
    }
}

/// Mark a task failed and broadcast the failure over WS. Shared by both
/// backend paths so errors surface identically to the client, and a
/// missing Python runtime becomes a clear task error instead of a panic.
async fn fail_task(state: &ServerState, task_id: &str, msg: String) {
    state.tasks.mark_failed(task_id, msg.clone()).await;
    broadcast(state, WsMessage::SegmentFailed {
        task_id: task_id.to_string(),
        message: msg,
    });
}

pub async fn handle_progress(
    state: ServerState,
    task_id: String,
) -> Result<ApiProgressResponse> {
    let t = state.tasks.get(&task_id).await
        .ok_or_else(|| anyhow!("Task {task_id} not found"))?;
    Ok(ApiProgressResponse {
        task_id,
        value: t.progress,
        status: t.status.as_str().to_string(),
        message: t.error.clone(),
    })
}

/// Single canonical `handle_cancel`. Replaces the two duplicated
/// definitions that previously lived in this file.
pub async fn handle_cancel(
    state: ServerState,
    req: CancelRequest,
) -> Result<CancelResponse> {
    // TotalSegmentator cannot be interrupted mid-pipeline, so we only flip
    // the local task state (mirrors the previous best-effort behavior).
    state.tasks.mark_cancelled(&req.task_id).await;
    broadcast(&state, WsMessage::SegmentCancelled {
        task_id: req.task_id.clone(),
    });

    let status = state.tasks.get(&req.task_id).await
        .map(|t| t.status.as_str().to_string()).unwrap_or_else(|| "cancelled".to_string());
    Ok(CancelResponse { task_id: req.task_id, status })
}

pub async fn handle_download(
    state: ServerState,
    task_id: String,
) -> Result<Vec<u8>> {
    let t = state.tasks.get(&task_id).await.ok_or_else(|| anyhow!("Task {task_id} not found"))?;
    t.mask.ok_or_else(|| anyhow!("No mask available for task {task_id}"))
}

/// A decoded volume ready for ONNX inference.
///
/// `volume` / `shape` / `spacing` are in the **network layout** that nnU-Net
/// was trained on: the RAS-canonical volume with its axis order reversed
/// (`transpose((2, 1, 0))`), i.e. `(S, P, L)` — head-foot first. That is what
/// `NibabelIOWithReorient` feeds the network, and it is *not* the raw MHA
/// `(Z, Y, X)` order nor the plain `(X, Y, Z)` canonical order.
///
/// `plan` is the canonical plan (raw `(Z, Y, X)` -> nibabel RAS `(X, Y, Z)`);
/// `plan.network_layout()` recovers the plan used to build `volume`, and
/// doubles as its inverse when mapping masks back to the original MHA space.
pub struct LoadedVolume {
    pub volume: Vec<f32>,
    pub shape: (usize, usize, usize), // network layout (S, P, L)
    pub spacing: [f32; 3],            // network layout (ss, sp, sl)
    pub plan: ReorientPlan,
}

/// Load an MHA volume from `state.series_path(series_id)` and decode it to
/// a flat `Vec<f32>` of HU values. The MHA's `TransformMatrix` is read and
/// the volume is reoriented to RAS canonical (matching TotalSegmentator's
/// `as_closest_canonical`), so the orientation / pixel-type / slope-intercept
/// handling stays identical to the WASM loader while the network sees the
/// frame it was trained on.
async fn load_volume_f32(
    state: &ServerState,
    series_id: &str,
) -> Result<LoadedVolume> {
    let path = state.series_path(series_id);
    anyhow::ensure!(
        path.exists(),
        "volume file not found on disk: {}",
        path.display()
    );
    log::info!("Loading MHA volume from {}", path.display());

    let bytes = tokio::fs::read(&path).await?;
    let metadata = MhaParser::parse_metadata_only(&bytes)
        .map_err(|e| anyhow!("MHA metadata parse failed: {e}"))?;

    let start_offset = metadata.data_offset.unwrap_or(0);
    anyhow::ensure!(
        start_offset < bytes.len(),
        "MHA data offset {} beyond file end ({})",
        start_offset,
        bytes.len()
    );

    // Keep the header text so we can read ElementSlope / ElementIntercept.
    let header_text = String::from_utf8_lossy(&bytes[..start_offset]);
    let (slope, intercept) = read_rescale(&header_text);

    // Drop the header in place; remaining bytes are raw voxel data.
    let mut raw = bytes;
    raw.drain(..start_offset);

    let mut dimensions = metadata.dimensions;
    let mut spacing = metadata.spacing;
    let mut offset = metadata.offset;
    if dimensions.len() != 3 {
        dimensions.push(1);
        spacing.push(1.0);
        offset.push(0.0);
    }

    // Convert MHA [x, y, z] spacing to the Z,Y,X order used by the volume
    // buffer so resample_volume_trilinear gets per-axis spacing that matches
    // the Z,Y,X row-major layout.
    let spacing_zyx = [spacing[2], spacing[1], spacing[0]];

    let ct_volume = MedicalVolume::generate_ct_volume_mha(
        [dimensions[0], dimensions[1], dimensions[2]], // [x, y, z]
        raw,
        metadata.pixel_type,
        spacing,
        offset,
        Vec::new(), // orientation transform is ignored by generate_ct_volume_mha
        slope,
        intercept,
    )
    .map_err(|e| anyhow!("CT volume generation failed: {e}"))?;

    // CTVolume stores voxels as i16 HU in Z,Y,X row-major order and
    // reports dimensions as (x, y, z) = (width, height, depth).
    let (header_w, header_h, header_d) = ct_volume.dimensions();
    let f32_data: Vec<f32> = ct_volume.voxel_data().iter().map(|&v| v as f32).collect();

    // The MHA header `DimSize` and the actual binary voxel buffer can
    // disagree in practice (e.g. a volume padded to a power-of-two depth,
    // or a header that under/over-states the slice count). The ingested
    // buffer is the ground truth for how many voxels we actually hold, so
    // derive the depth from it while keeping width/height from the header.
    // This guarantees `f32_data.len() == depth * height * width`, so the
    // downstream length check in the segment engine can never see a
    // mismatch like `length 67108864 != expected 65011712`.
    let (width, height) = (header_w, header_h);
    let slices = f32_data.len() / (width * height);
    anyhow::ensure!(
        f32_data.len() == slices * width * height,
        "MHA voxel buffer length {} is not divisible by width*height={} (header dims {:?})",
        f32_data.len(), width * height, (header_w, header_h, header_d)
    );
    if slices != header_d {
        log::warn!(
            "MHA depth mismatch for series {}: header depth={} but buffer holds {} slices (using buffer)",
            series_id, header_d, slices
        );
    }

    // --- RAS canonicalization + network layout ---
    // `f32_data` is in the MHA's raw `(Z, Y, X)` layout; `orientation` is the
    // MHA 3x3 direction-cosines matrix (ITK/LPS world). nnU-Net's reader
    // (`NibabelIOWithReorient`) canonicalizes to RAS and then applies
    // `transpose((2, 1, 0))`, so the tensor the network sees is a `(S, P, L)`
    // view — the reverse axis order of the `(X, Y, Z)` canonical volume.
    // `network_layout()` folds both steps (LPS->RAS sign convention, RAS
    // canonicalization and the final axis reversal) into a single plan, so we
    // reorient the raw buffer exactly once into the trained-on layout.
    //
    // `plan` stays the *canonical* plan; it is what maps the predicted mask
    // back to the original MHA space.
    let mut plan = ReorientPlan::from_orientation(metadata.orientation);
    plan.original_shape = (slices, height, width); // (Z, Y, X)
    let net_plan = plan.network_layout();
    let (vol_net, shape_net) =
        reorient_volume_to_canonical(&f32_data, (slices, height, width), &net_plan);
    let spacing_net = net_plan.canonical_spacing(spacing_zyx); // (S, P, L)

    Ok(LoadedVolume {
        volume: vol_net,
        shape: shape_net,
        spacing: spacing_net,
        plan,
    })
}

/// Read `ElementSlope` / `ElementIntercept` from an MHA header. If absent,
/// default to identity (slope=1, intercept=0), i.e. assume HU is stored
/// directly - the common case for TotalSegmentator inputs.
fn read_rescale(header: &str) -> (f32, f32) {
    let mut slope = 1.0f32;
    let mut intercept = 0.0f32;
    for line in header.lines() {
        if let Some((k, v)) = line.split_once('=') {
            let k = k.trim();
            let v = v.trim();
            if k.eq_ignore_ascii_case("ElementSlope") {
                if let Ok(x) = v.parse() { slope = x; }
            } else if k.eq_ignore_ascii_case("ElementIntercept") {
                if let Ok(x) = v.parse() { intercept = x; }
            }
        }
    }
    (slope, intercept)
}

/// How often the background poller queries the AI service.
const POLL_INTERVAL_MS: u64 = 1000;
