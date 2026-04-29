# View System and Layout System (Implementation Notes)

This document describes what is implemented today under `src/rendering/view/` and how the pieces fit together. It is intended as a “map” for the view subsystem rather than a design proposal.

Related design docs (higher-level intent):
- `doc/views/mpr-view-design.md`
- `doc/views/mip-view-design.md`

## Module map (source code)

- `src/rendering/view/mod.rs`
  - Central module entry point and re-exports (View traits, layouts, factories, MPR/MIP/Mesh modules, RenderContent).
- `src/rendering/view/renderable.rs`
  - Minimal `Renderable` trait: `update(queue)` and `render(render_pass)`.
- `src/rendering/view/view.rs`
  - Core traits and types: `View`, `StatefulView`, `ViewState`, `Orientation`.
- `src/rendering/view/layout.rs`
  - Layout strategies + containers:
    - `LayoutStrategy` returns `(pos, size)` for each view slot
    - `StaticLayout<T>` (strategy fixed at compile time)
    - `DynamicLayout` (strategy can change at runtime)
    - `compute_aspect_fit` helper for aspect-preserving placement inside a view rect
- `src/rendering/view/view_factory.rs`
  - `ViewFactory` trait + concrete `DefaultViewFactory` (GPU-backed) + `MockViewFactory` (tests).
- `src/rendering/view/view_manager.rs`
  - `ViewManager`: state save/restore registry + factory-forwarding helpers.
- `src/rendering/view/render_content.rs`
  - `RenderContent`: owns a 3D texture + view + sampler + `VolumeEncoding`, plus helpers to upload data.
- `src/rendering/view/mpr/`
  - MPR view implementation and shared rendering context (`MprView`, `MprRenderContext`, etc.).
- `src/rendering/view/mip/`
  - MIP view implementation (`MipView`, `MipViewWgpuImpl`, uniforms, pipeline wiring).
- `src/rendering/view/mesh/`
  - Mesh view implementation (`MeshView`) and helpers/contexts.

## Core traits: Renderable and View

### Renderable

`Renderable` is the minimal interface required by the render loop:
- `update(&mut self, queue: &wgpu::Queue)`
- `render(&mut self, render_pass: &mut wgpu::RenderPass) -> Result<(), wgpu::SurfaceError>`

Views (and layout containers) implement `Renderable` so the render loop can treat them uniformly.

### View

`View` extends `Renderable` and adds:
- geometry placement: `position()`, `dimensions()`, `move_to(pos)`, `resize(dim)`
- runtime type inspection: `as_any()`, `as_any_mut()` for downcasting when necessary

### StatefulView and ViewState

`StatefulView` allows saving and restoring a view’s user-facing parameters via `ViewState`.

`ViewState` is focused on medical-imaging workflows (window/level, slice position, zoom/pan, and view rect). This enables switching between view types without losing context.

## Layout: strategies and containers

Layout is responsible for positioning and sizing `View` instances within a parent area.

- `LayoutStrategy::calculate_position_and_size(index, total_views, parent_dim) -> (pos, size)`
  - Strategies compute pixel-space rects.
  - The container calls `move_to` and `resize` on views accordingly.

Two container implementations exist:
- `StaticLayout<T>`: strategy is a concrete type parameter.
- `DynamicLayout`: strategy is a boxed trait object, enabling runtime strategy switching.

Common strategies include:
- `GridLayout`: rows × cols grid with spacing.
- `OneCellLayout`: shows the first view, hides the rest using a minimal 1×1 viewport.
- `LargeLeft3RightLayout`: one large view on the left and three stacked views on the right.

Aspect-fit helper:
- `compute_aspect_fit(container_w, container_h, content_w, content_h, padding) -> Option<AspectFitResult>`
  - Computes a centered letterboxed/pillarboxed inner rect to preserve content aspect ratio.

## RenderContent: GPU volume content shared across views

`RenderContent` owns the GPU resources needed for sampling a volume in shaders:
- `wgpu::Texture` (D3)
- `wgpu::TextureView`
- `wgpu::Sampler`
- `wgpu::TextureFormat`
- `VolumeEncoding` (how data is encoded in the texture)

Two upload paths are supported:
- Packed `Rg8Unorm` (with HU bias/offset decoding in shader)
- `R16Float` (half-float HU)

`RenderContent::decode_parameters()` returns a compact uniform-friendly struct so shaders can decode consistently.

## ViewFactory and ViewManager

Creation is centralized via `ViewFactory`:
- Ensures views are constructed with the correct device/queue/surface format and content encoding.
- Supports “with_content” variants so MPR/MIP can reuse the same `Arc<RenderContent>`.

`DefaultViewFactory` is the concrete GPU-backed factory.

`ViewManager` provides:
- a registry of saved `ViewState` snapshots (keyed by string position identifiers)
- helper methods that forward to the underlying `ViewFactory` for creating Mesh/MPR/MIP views
- state restoration helpers for views that support `StatefulView` (currently downcast-based)

## Notes on documentation scope

This file documents the “what exists today” implementation. Design intent and math details should stay in:
- `doc/views/mpr-view-design.md`
- `doc/views/mip-view-design.md`
