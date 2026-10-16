# Mesh View Factory: `create_mesh_view` accepts a mesh parameter

Time: 2026-04-29T09-54-46

Summary:
- `ViewFactory::create_mesh_view` now accepts a `&CTVolume` parameter so callers can provide their own geometry when creating mesh views.
- This change mirrors the additive design for MPR/MIP where callers can inject prebuilt GPU resources, enabling better performance and flexibility for clinical workflows.

Details:
- New trait signature:
  `fn create_mesh_view(&self, vol: &CTVolume, pos: (i32, i32), size: (u32, u32)) -> Result<Box<dyn View>, Box<dyn std::error::Error>>`.
- `DefaultViewFactory::create_mesh_view` uses the provided volume to build a fresh `BasicMeshContext` with depth testing enabled; rotation is enabled by default for better inspection.
- `ViewManager::create_mesh_view` retains its existing API by constructing a default `CTVolume::new()` internally and forwarding to the factory. Existing callers of `ViewManager` do not need changes.

Why:
- Medical imaging applications frequently render different anatomical or tool volumes. Accepting a volume parameter avoids redundant volume construction and accommodates caller-provided geometry.
- Keeps GPU operations efficient by building buffers directly from the input volume and avoids unnecessary CPU–GPU sync.
- Consistent with MPR/MIP "with_content" variants that enable resource reuse.

Usage example (no_run):

```rust,no_run
use std::sync::Arc;
use kepler_wgpu::rendering::view::{DefaultViewFactory, ViewFactory};
use kepler_wgpu::data::ct_volume::CTVolume;

// Assume you have initialized device, queue, and surface_format
let device: Arc<wgpu::Device> = unimplemented!("device setup");
let queue: Arc<wgpu::Queue> = unimplemented!("queue setup");
let surface_format: wgpu::TextureFormat = wgpu::TextureFormat::Bgra8UnormSrgb;
let factory = DefaultViewFactory::new(device, queue, surface_format, /*use_float_volume_texture=*/ true);

// Provide your own volume (can be custom geometry or built-in helper)
let vol = CTVolume::new();

// Create a mesh view at position (0,0) sized 512x512
let view = factory.create_mesh_view(&vol, (0, 0), (512, 512))
    .expect("mesh view creation");
```

Notes:
- Orthogonal projection remains the default (no perspective unless explicitly requested) to maintain medical imaging accuracy.
- Heavy TRACE logging is disabled in release builds and gated behind the `trace-logging` feature; use sampling if enabling TRACE within render loops.
- In wasm builds, logs route to the browser console.