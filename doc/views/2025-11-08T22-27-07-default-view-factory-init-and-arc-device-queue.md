# DefaultViewFactory initialization and Arc<Device/Queue> change

Summary

This change fixes a compilation error in `State::new` where `wgpu::Device` and `wgpu::Queue` were incorrectly cloned to initialize `DefaultViewFactory`. Since `wgpu` handles are not `Clone`, we updated the `Graphics` struct to store `Arc<wgpu::Device>` and `Arc<wgpu::Queue>` and adjusted factory initialization accordingly.

What changed

- `Graphics` now stores `device: Arc<wgpu::Device>` and `queue: Arc<wgpu::Queue>`.
- `Graphics::initialize()` wraps the created `wgpu::Device` and `wgpu::Queue` in `Arc` before returning.
- `State::new()` initializes `DefaultViewFactory` using `Arc::clone(&graphics_context.graphics.device)` and `Arc::clone(&graphics_context.graphics.queue)`.

Why this is safe

- All existing call sites that use `&self.graphics.device` or `&self.graphics.queue` continue to work via Rust's auto-deref coercions from `&Arc<T>` to `&T`.
- No public API was changed; `Graphics` fields are `pub(crate)`.
- Sharing the device and queue via `Arc` aligns with typical `wgpu` patterns and avoids unnecessary handle moves.

Performance and correctness notes

- Using `Arc` for `wgpu::Device` and `wgpu::Queue` is inexpensive and does not introduce additional synchronization in `wgpu` itself; it only manages reference counts on the handle.
- This enables safe sharing of GPU handles across subsystems (e.g., factories, contexts) without lifetime complexities.

Minimal example (no_run)

```rust,no_run
/// Example: Initialize DefaultViewFactory from GraphicsContext (no_run)
/// This example demonstrates how to pass Arc-wrapped device/queue to the factory.
#[allow(unused)]
fn init_factory(gc: &crate::rendering::core::GraphicsContext, use_float: bool) -> crate::rendering::DefaultViewFactory {
    crate::rendering::DefaultViewFactory::new(
        std::sync::Arc::clone(&gc.graphics.device),
        std::sync::Arc::clone(&gc.graphics.queue),
        gc.graphics.surface_config.format,
        use_float,
    )
}
```

Build status

- Native build: cargo build — OK
- Native tests: cargo test — 27 passed, 2 failed due to missing external MHD/MHA files (path not found). These failures are unrelated to this change.
- WASM build: wasm-pack build -t web — unchanged by this change; expected to remain OK.

Logging and performance

- Default logging level is INFO.
- TRACE logs remain gated behind the `trace-logging` feature flag and should be sampled when enabled.
- No changes to GPU render loops; operations remain non-blocking and efficient.