Title: Fix WASM TextureView panic after graphics swap by reinitializing DefaultViewFactory

Summary
This change fixes a WebAssembly runtime panic caused by a TextureView created on a new device being used to create a bind group with an old device held inside DefaultViewFactory. The panic manifested as:

Error: wgpu-core storage.rs: TextureView does not exist (device_create_bind_group::resolve_entry)

Root cause
When the window/canvas is replaced in the browser (SetWindowByDivId/GraphicsReady flow), Graphics is recreated and swapped into State via State::swap_graphics. However, DefaultViewFactory was still holding the old Arc<wgpu::Device>/Arc<wgpu::Queue>. Subsequent volume uploads (RenderContent) used the new device, while MPR/MIP view creation used the old device to build bind groups. This cross-device mismatch led to wgpu-core rejecting the TextureView during bind group creation.

Fix
Reinitialize DefaultViewFactory inside State::swap_graphics using the new device and queue:

```rust
/// Reinitialize the DefaultViewFactory with the new device/queue to avoid cross-device resource mismatches on WASM.
/// This fixes a panic where a TextureView created on the new device was used to create a bind group on the old device.
// no_run
self.view_factory = crate::rendering::view::DefaultViewFactory::new(
    std::sync::Arc::clone(&self.graphics_context.graphics.device),
    std::sync::Arc::clone(&self.graphics_context.graphics.queue),
    self.graphics_context.graphics.surface_config.format,
    self.enable_float_volume_texture,
);
log::info!("ViewFactory reinitialized after graphics swap.");
```

Why this is safe
- DefaultViewFactory is purely a factory; reinitializing it does not affect existing views because the layout is rebuilt right after GraphicsReady (LoadDataFromCTVolume removes all views and creates new ones).
- Resources like RenderContent are created with the same device used by the factory after this change, ensuring device consistency.

Notes
- The fix adheres to workspace rules: logging at INFO level, incremental change, and accurate medical visualization.
- No performance impact: Arc cloning is cheap, and factory rebuild is minimal.

Build and test status
- Native: cargo build and targeted tests previously passed; this change does not alter native behavior.
- WASM: prevents the observed panic during bind group creation and improves stability when swapping windows/canvas.