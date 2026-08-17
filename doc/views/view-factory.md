# ViewFactory and DefaultViewFactory (Consolidated Notes)

## What this document covers

This file consolidates the ViewFactory-related notes that were previously spread across multiple timestamped docs.

It describes:
- Why `ViewFactory` exists and where it lives
- The current `ViewFactory` API surface (Mesh/MPR/MIP + `*_with_content` variants)
- The concrete `DefaultViewFactory` implementation (volume texture encoding paths, device/queue ownership)
- How `ViewManager` forwards view creation
- A WASM-specific gotcha when swapping graphics/device

## Why ViewFactory was extracted

- `ViewFactory` is defined in `src/rendering/view/view_factory.rs` and re-exported via `src/rendering/view/mod.rs`.
- This keeps construction logic separate from the core view traits and types (`View`, `StatefulView`, `Orientation`, `ViewState`), improving testability and keeping module boundaries clean.

## Current API

### Core creation methods

- `create_mesh_view(&self, mesh: &Mesh, pos: (i32, i32), size: (u32, u32)) -> Result<Box<dyn View>, Box<dyn Error>>`
- `create_mpr_view(&self, vol: &CTVolume, orientation: Orientation, pos: (i32, i32), size: (u32, u32)) -> Result<Box<dyn View>, Box<dyn Error>>`
- `create_mip_view(&self, vol: &CTVolume, pos: (i32, i32), size: (u32, u32)) -> Result<Box<dyn View>, Box<dyn Error>>`

### RenderContent reuse variants

For MPR and MIP, there are `*_with_content` variants that accept `Arc<RenderContent>`. These exist to avoid repeated volume conversion + GPU upload when building multiple views for the same dataset.

- `create_mpr_view_with_content(render_content: Arc<RenderContent>, vol: &CTVolume, orientation: Orientation, pos: (i32, i32), size: (u32, u32))`
  - Still requires `&CTVolume` for metadata (dimensions/spacing) used in slicing math.
- `create_mip_view_with_content(render_content: Arc<RenderContent>, pos: (i32, i32), size: (u32, u32))`

There is also a mesh-oriented variant for API symmetry:

- `create_mesh_view_with_content(render_content: Arc<RenderContent>, pos: (i32, i32), size: (u32, u32))`

## DefaultViewFactory: concrete GPU-backed implementation

`DefaultViewFactory` owns:
- `Arc<wgpu::Device>`
- `Arc<wgpu::Queue>`
- `wgpu::TextureFormat` (surface format)
- `use_float_volume_texture: bool` (selects volume texture encoding path)

### Volume encoding paths (RenderContent)

`DefaultViewFactory` builds `RenderContent` on-demand from `CTVolume` using one of two encoding paths:

- R16Float (half-float) path:
  - Converts voxel `i16` to `f16` bit patterns
  - Uploads into `wgpu::TextureFormat::R16Float`
  - Uses `VolumeEncoding::HuFloat`
- Packed RG8 path:
  - Applies HU offset (`VolumeEncoding::DEFAULT_HU_OFFSET`) and packs into `u16`
  - Uploads into `wgpu::TextureFormat::Rg8Unorm`
  - Uses `VolumeEncoding::HuPackedRg8 { offset }`

These encodings are exposed via `RenderContent::decode_parameters()` so shaders can decode back into HU consistently.

## ViewManager forwarding

`ViewManager` provides convenience methods like `create_mpr_view` / `create_mip_view` / `create_mesh_view` that forward to the configured `ViewFactory`.

One ergonomics detail:
- `ViewManager::create_mesh_view` keeps a no-argument external API and constructs a default demo mesh (`Mesh::spine_vertebra()`), then forwards to `ViewFactory::create_mesh_view(&mesh, ...)`.

## WASM gotcha: swapping graphics/device requires reinitializing the factory

When running in WebAssembly, the canvas/window may be replaced and WGPU can recreate the device/queue. If `DefaultViewFactory` still holds the old `Arc<wgpu::Device>/Arc<wgpu::Queue>`, then:
- new `RenderContent` may be created with the new device, while
- view creation/bind groups may still use the old device,

leading to a cross-device mismatch and a runtime panic during bind group creation.

Mitigation:
- whenever graphics/device is swapped, reinitialize `DefaultViewFactory` with the new device/queue.

## Usage example

```rust
// no_run
use kepler_wgpu::rendering::view::{Orientation, View, ViewFactory};
use kepler_wgpu::rendering::view::mesh::mesh::Mesh;
use kepler_wgpu::CTVolume;

fn create_views(
    factory: &impl ViewFactory,
    vol: &CTVolume,
) -> Result<(Box<dyn View>, Box<dyn View>), Box<dyn std::error::Error>> {
    let mesh = Mesh::spine_vertebra();
    let mesh_view = factory.create_mesh_view(&mesh, (0, 0), (512, 512))?;
    let mpr_view = factory.create_mpr_view(vol, Orientation::Transverse, (512, 0), (512, 512))?;
    Ok((mesh_view, mpr_view))
}
```
