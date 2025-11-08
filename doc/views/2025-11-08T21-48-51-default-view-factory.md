# DefaultViewFactory Implementation

Short explanation of the newly added concrete factory for view creation.

## Summary
The `DefaultViewFactory` provides a centralized, GPU-backed creation path for `MeshView`, `MprView`, and `MipView`. It holds the WGPU `Device`, `Queue`, and the target `TextureFormat`, and supports both float (R16Float) and packed RG8 volume textures via `RenderContent` helpers.

## Design Notes
- Reuses existing constructors and contexts:
  - `MeshView` with a fresh `BasicMeshContext` and default rotation enabled.
  - `MprView` initialized with an `Arc<MprRenderContext>` and `RenderContent` built from `CTVolume` voxel data.
  - `MipView` via `MipViewWgpuImpl::new(render_content, device, surface_format)` with zero-copy `Arc<RenderContent>` sharing.
- Logging levels follow workspace rules (INFO/DEBUG; no TRACE by default).
- Efficient GPU resource usage: shared contexts and minimized copies.
- Cross-platform: compiles for native and WebAssembly builds.

## Example Usage
```rust
//! no_run
use std::sync::Arc;
use kepler_wgpu::rendering::DefaultViewFactory;

fn create_factory(device: Arc<wgpu::Device>, queue: Arc<wgpu::Queue>, surface_format: wgpu::TextureFormat) -> DefaultViewFactory {
    // Use RG8 packed path by default; set to true for R16Float volume textures
    let use_float_volume = false;
    DefaultViewFactory::new(device, queue, surface_format, use_float_volume)
}
```

## Incremental Development
This is a minimal viable implementation that wires existing view constructors without altering public APIs. Future iterations can add configuration knobs (e.g., presets, sampling strategies) while preserving performance and numerical stability required by medical imaging workflows.