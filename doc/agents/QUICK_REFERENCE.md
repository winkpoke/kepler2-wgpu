# Quick Reference

**Last Updated**: 2025-01-15

## Common Commands

### Native Development

```bash
# Run application
cargo run

# Build release
cargo build --release

# Run with logging
RUST_LOG=debug cargo run

# Run tests
cargo test
```

### WASM Development

```bash
# Build for web
wasm-pack build --target web

# Serve static files
npx live-server ./static
```

### Code Quality

```bash
# Format code
cargo fmt

# Lint
cargo clippy

# Type check
cargo check
```

## Environment Variables

```bash
# Logging level
RUST_LOG=info          # error, warn, info, debug, trace

# GPU backend
KEPLER_WGPU_BACKEND=vulkan  # dx12, vulkan, metal, gl, primary

# GPU validation
KEPLER_WGPU_VALIDATION=true
```

## File Structure

```
src/
├── core/           # Utilities, errors, types (no dependencies)
├── data/           # DICOM, CT volumes (depends on core)
├── rendering/      # WGPU, views, shaders (depends on core, data)
└── application/    # UI, events (depends on all)
```

## Key Files

| File | Purpose |
|------|---------|
| `src/lib.rs` | Public API, logger init, WASM entry |
| `src/main.rs` | Native binary entry |
| `src/application/app.rs` | Main app state |
| `src/core/error.rs` | Error types (`KeplerError`) |
| `src/data/ct_volume.rs` | CT volume struct |
| `src/rendering/core/graphics.rs` | GPU initialization |

## Platform-Specific Code

```rust
// WASM only
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

// Native only
#[cfg(not(target_arch = "wasm32"))]
use tokio::runtime::Runtime;
```

## Async Patterns

```rust
// WASM
#[cfg(target_arch = "wasm32")]
use async_lock::Mutex;

// Native
#[cfg(not(target_arch = "wasm32"))]
use tokio::sync::Mutex;
```

## Error Handling

```rust
use crate::core::error::{KeplerError, KeplerResult};

pub fn process_data() -> KeplerResult<ProcessedData> {
    Ok(data)
}
```

## Logging

```rust
use log::{info, warn, error, debug};

info!("Application started");
error!("Failed: {}", e);
```

## Coordinate Systems

- **World**: 3D world coordinates (mm from volume origin)
- **Screen**: 2D screen coordinates (pixels, top-left origin)
- **Voxel**: 3D voxel indices (integer indices)
- **Base**: Base coordinate type (`glam::Vec3`)

```rust
use glam::Mat4, Vec3;
let transform = Mat4::from_translation(Vec3::new(x, y, z));
```

## Common Gotchas

❌ Don't use `tokio` in WASM modules
❌ Don't clone `wgpu::Surface` or `TextureView`
❌ Don't suppress type errors with `as any`
❌ Don't mix OpenGL/WebGL with WebGPU

✅ Check GPU capabilities before using R16Float textures
✅ Recreate pipelines after surface format changes
✅ Use `pollster` to block on async in tests
✅ Update `src/lib.rs` when changing public API

## Test Patterns

```rust
#[test]
fn test_functionality() {
    let result = tested_function();
    assert_eq!(result, expected);
}

#[test]
#[cfg(not(target_arch = "wasm32"))]
fn test_native_only() {
    // Native-only test
}
```

## Commit Format

```
feat(scope): description
fix(scope): description
refactor(scope): description
test(scope): description
docs(scope): description
```

## PR Checklist

- [ ] Code formatted (`cargo fmt`)
- [ ] No clippy warnings
- [ ] Tests pass (`cargo test`)
- [ ] Native build succeeds
- [ ] WASM build succeeds
- [ ] Public API updated in `src/lib.rs`
- [ ] Documentation updated

## OpenSpec Workflow

```bash
# List changes
openspec list

# List specs
openspec list --specs

# Validate change
openspec validate [change-id] --strict

# Archive change
openspec archive [change-id] --yes
```

## Canvas IDs

- WASM canvas ID: `wasm-example` (in `static/index.html`)
- Canvas size controlled by CSS on WASM, Winit on native

## Dependencies

### Key Libraries

- `wgpu` (23.0): Cross-platform graphics
- `winit` (0.29): Window and event loop
- `glam` (0.30): Vector and matrix math
- `dicom-*` crates: DICOM parsing
- `wasm-bindgen`: WASM/JS interop

### Native Only

- `tokio`: Async runtime
- `rayon`: Parallel processing

### WASM Only

- `console_log`: Browser logging
- `web-sys`: Web API bindings

## GPU Resource Lifecycle

Drop order: textures → buffers → bind groups → pipelines

## Common Issues

### WASM build fails
```bash
rustup target add wasm32-unknown-unknown
cargo install wasm-pack
```

### Surface errors
```bash
KEPLER_WGPU_BACKEND=vulkan cargo run
KEPLER_WGPU_VALIDATION=true cargo run
```

### Tests hang
Use `pollster` to block on async (already configured)

### Logging not showing in WASM
Check browser console (F12)

## Performance Tips

- Reuse bind groups and buffers
- Use `Arc` for Device and Queue (already wrapped)
- Avoid cloning Surface or TextureView
- Batch GPU operations
- Use texture pools for dynamic resources

## Getting Help

- See `ARCHITECTURE.md` for project structure
- See `CONVENTIONS.md` for coding standards
- See `RENDERING.md` for GPU patterns
- See `PITFALLS.md` for anti-patterns
- See `PR_GUIDELINES.md` for contribution workflow
- See `OPENSPEC.md` for spec-driven development

## Quick Links

- **Architecture**: `doc/agents/ARCHITECTURE.md`
- **Conventions**: `doc/agents/CONVENTIONS.md`
- **Build & Test**: `doc/agents/BUILD.md`
- **Rendering**: `doc/agents/RENDERING.md`
- **Pitfalls**: `doc/agents/PITFALLS.md`
- **PR Guidelines**: `doc/agents/PR_GUIDELINES.md`
- **OpenSpec**: `doc/agents/OPENSPEC.md`
