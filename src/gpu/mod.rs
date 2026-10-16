//! Native-only GPU backend boundary.
//!
//! The viewer and compute backend both use wgpu 30.0.1. Native compute retains
//! the crate-local `wgpu30` alias and its independent `GpuState` / `GpuContext`
//! device ownership; it never borrows the viewer's surface or GPU resources.
//!
//! Everything here is gated to `#[cfg(not(target_arch = "wasm32"))]` so the WASM
//! build excludes the server compute owner while its viewer uses wgpu 30's
//! WebGPU/WebGL backends.

#[cfg(not(target_arch = "wasm32"))]
mod native;
#[cfg(not(target_arch = "wasm32"))]
mod compute;

#[cfg(not(target_arch = "wasm32"))]
pub use native::{GpuContext, GpuState, GpuVolume};
#[cfg(not(target_arch = "wasm32"))]
pub use compute::{run_double, run_volume_upload, prepare_volume_upload, run_volume_compute};
