//! Native-only GPU backend boundary.
//!
//! This module establishes a clean separation between the existing WASM/browser
//! rendering path (which uses `wgpu` **23** through the `rendering` stack) and a
//! future native compute path (which uses `wgpu` **30**, aliased as `wgpu30`
//! in `Cargo.toml`). The two backends intentionally do **not** share one unified
//! wgpu API layer; they only share the conceptual `GpuState` / `GpuContext`
//! boundary so the native compute backend can evolve independently toward wgpu 30.
//!
//! Everything here is gated to `#[cfg(not(target_arch = "wasm32"))]` so the WASM
//! build continues to resolve `wgpu` 23 (with the `webgl` feature) and never sees
//! `wgpu` 30 types or APIs.

#[cfg(not(target_arch = "wasm32"))]
mod native;
#[cfg(not(target_arch = "wasm32"))]
mod compute;

#[cfg(not(target_arch = "wasm32"))]
pub use native::{GpuContext, GpuState, GpuVolume};
#[cfg(not(target_arch = "wasm32"))]
pub use compute::{run_double, run_volume_upload, prepare_volume_upload, run_volume_compute};
