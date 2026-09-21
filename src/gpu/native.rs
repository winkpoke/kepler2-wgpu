//! Native GPU context backed by `wgpu` **30** (aliased as `wgpu30` in Cargo.toml).
//!
//! Deliberately decoupled from the WASM/browser rendering path. It only
//! initializes a headless, compute-capable device — no `Surface`, no `Window`,
//! no `winit`. The first phase of the native backend targets compute only.

use std::sync::Arc;

use anyhow::{Context, Result};
use parking_lot::Mutex;

/// A lazily-created, shared native GPU context.
///
/// `device`/`queue` come from `wgpu` 30 and are kept entirely inside this module;
/// the rest of the crate must not mix them with the `wgpu` 23 types used by the
/// rendering stack.
pub struct GpuContext {
    pub(crate) adapter_name: String,
    pub(crate) backend: String,
    pub(crate) device: wgpu30::Device,
    pub(crate) queue: wgpu30::Queue,
}

impl GpuContext {
    /// Request the best available adapter and create a compute-capable device.
    ///
    /// `compatible_surface` is `None` on purpose: the native backend starts with
    /// compute only and does not need a window/surface.
    pub async fn new() -> Result<Arc<Self>> {
        let instance = wgpu30::Instance::default();

        let adapter = instance
            .request_adapter(&wgpu30::RequestAdapterOptions {
                power_preference: wgpu30::PowerPreference::HighPerformance,
                compatible_surface: None,
                force_fallback_adapter: false,
                ..Default::default()
            })
            .await
            .context("failed to request a GPU adapter (is a compatible backend present?)")?;

        let (device, queue) = adapter
            .request_device(
                &wgpu30::DeviceDescriptor {
                    label: Some("kepler-native-gpu"),
                    required_features: wgpu30::Features::empty(),
                    required_limits: wgpu30::Limits::default(),
                    ..Default::default()
                })
            .await
            .context("failed to request a GPU device")?;

        let info = adapter.get_info();
        let adapter_name = info.name.clone();
        let backend = format!("{:?}", info.backend);

        log::info!("GPU (native wgpu 30): {adapter_name} | Backend: {backend}");

        Ok(Arc::new(Self {
            adapter_name,
            backend,
            device,
            queue,
        }))
    }

    pub fn adapter_name(&self) -> &str {
        &self.adapter_name
    }

    pub fn backend(&self) -> &str {
        &self.backend
    }

    pub fn device(&self) -> &wgpu30::Device {
        &self.device
    }

    pub fn queue(&self) -> &wgpu30::Queue {
        &self.queue
    }
}

/// Lazily-initialized, shared GPU state for the Axum server.
///
/// Lives inside `ServerState` only on the native backend
/// (`cfg(not(target_arch = "wasm32"))`), so the WASM build never references
/// `wgpu30` or any `wgpu` 30 type.
pub struct GpuState {
    inner: Mutex<Option<Arc<GpuContext>>>,
}

impl GpuState {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(None),
        }
    }

    /// Returns the shared GPU context, initializing it on first use.
    ///
    /// Concurrent callers may each end up initializing once; the winner is stored
    /// and later callers reuse it. This is acceptable for a smoke-test endpoint.
    pub async fn context(&self) -> Result<Arc<GpuContext>> {
        if let Some(ctx) = self.inner.lock().as_ref() {
            return Ok(ctx.clone());
        }
        let ctx = GpuContext::new().await?;
        *self.inner.lock() = Some(ctx.clone());
        Ok(ctx)
    }
}

impl Default for GpuState {
    fn default() -> Self {
        Self::new()
    }
}

/// Reserved interface boundary for a future `CTVolume` <-> GPU mapping.
///
/// **Not implemented in this change.** `run_volume_upload` (in `compute.rs`) is
/// the only piece wired so far. This trait documents the intended contract so the
/// eventual CTVolume integration has a stable surface to target, without pulling
/// `CTVolume` / `voxel_data` into the native GPU module today.
pub trait GpuVolume {
    /// Upload voxel data to the device. Must not clone the caller's `Vec<i16>` —
    /// accept a borrowed slice and let the GPU backend copy it.
    fn upload(&self, ctx: &GpuContext, voxels: &[i16]) -> Result<()>;
    /// Read back a small bounded prefix for integrity checks (never the whole
    /// volume). Returns at most `n` voxels.
    fn read_sample(&self, ctx: &GpuContext, n: usize) -> Result<Vec<i16>>;
}
