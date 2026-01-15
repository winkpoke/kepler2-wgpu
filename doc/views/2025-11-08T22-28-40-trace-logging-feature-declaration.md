# Trace logging feature declaration

Summary

To comply with workspace rules requiring TRACE logging to be gated by a feature flag named `trace-logging`, we added the `trace-logging` feature in `Cargo.toml`.

What changed

- `Cargo.toml` now declares:

```toml
[features]
default = []
trace-logging = []
```

Usage

- Native builds: enable feature with `cargo run --features trace-logging` or `cargo test --features trace-logging` when you need detailed diagnostics.
- WASM builds: enable feature with `wasm-pack build -t web -- --features trace-logging` when debugging in the browser console.

Notes

- Keep TRACE logging disabled in release builds to avoid performance impact.
- Consider sampling (e.g., log every Nth frame) when TRACE logs are enabled to prevent flooding. Ensure heavy trace logs are guarded with `cfg(feature = "trace-logging")`.

Minimal example (no_run)

```rust,no_run
#[cfg(feature = "trace-logging")]
fn log_frame(frame_id: u64) {
    // Sample every 10th frame
    if frame_id % 10 == 0 {
        log::trace!("Frame {} diagnostics", frame_id);
    }
}
```