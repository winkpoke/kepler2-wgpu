# Coding Conventions

**Last Updated**: 2025-01-15

## Naming & Formatting

- **Rust Standard**: `PascalCase` for types, `snake_case` for functions/variables
- **Indentation**: 4 spaces (enforced by `rustfmt`)
- **Line Length**: 100 characters (soft limit)

## Platform-Specific Code

Use `#[cfg(target_arch = "wasm32")]` for platform separation:

```rust
// WASM-only code
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

// Native-only code
#[cfg(not(target_arch = "wasm32"))]
use tokio::runtime::Runtime;

// Platform-specific implementations
#[cfg(target_arch = "wasm32")]
pub fn platform_specific() {
    // WASM implementation
}

#[cfg(not(target_arch = "wasm32"))]
pub fn platform_specific() {
    // Native implementation
}
```

## Async Patterns

Cross-platform async synchronization:

```rust
// WASM: use async_lock::Mutex
#[cfg(target_arch = "wasm32")]
use async_lock::Mutex;

// Native: use tokio::sync::Mutex
#[cfg(not(target_arch = "wasm32"))]
use tokio::sync::Mutex;
```

**IMPORTANT**: Never import `tokio` in WASM-targeted modules.

## Error Handling

Use `KeplerResult<T>` type alias (`Result<T, KeplerError>`):

```rust
use crate::core::error::{KeplerError, KeplerResult};

pub fn process_data() -> KeplerResult<ProcessedData> {
    // ...
    Ok(data)
}
```

Error types defined in `src/core/error.rs`:
- `KeplerError::Graphics(String)`
- `KeplerError::Dicom(String)`
- `KeplerError::Io(std::io::Error)`
- `KeplerError::Surface(wgpu::SurfaceError)`
- `KeplerError::Window(String)`
- `KeplerError::Validation(String)`
- `KeplerError::Mpr(MprError)`

## Logging

Always use crate-level `log` macros for cross-platform logging:

```rust
use log::{info, warn, error, debug};

info!("Application started");
warn!("Missing parameter: {}", param);
error!("Failed to load volume: {}", e);
debug!("Rendering frame: {}", frame_count);
```

**Logger Initialization** (in `src/lib.rs`):
- **Native**: `env_logger` with wgpu modules filtered to WARN level
- **WASM**: Custom logger routing to `web_sys::console`

**Do NOT use**:
- `println!`, `eprintln!` (except in `main.rs` for quick debugging)
- `dbg!` in production code

## Re-exports

`src/lib.rs` re-exports commonly used types. When changing public API, update `lib.rs` re-exports accordingly:

```rust
// Example re-export pattern
pub use application::{render_app::RenderApp, gl_canvas::GLCanvas};
pub use data::ct_volume::CTVolume;
pub use core::error::{KeplerError, KeplerResult};
```

## Test Patterns

### Test Organization

```
tests/
├── dicom_tests.rs              # DICOM parsing and validation
├── medical_imaging_tests.rs     # MHA/MHD format tests
├── mesh_integration_tests.rs    # 3D mesh rendering tests
├── mpr_view_validation_tests.rs # MPR view correctness tests
├── performance_tests.rs         # Performance benchmarks
└── error_handling_tests.rs     # Error handling edge cases
```

### Platform Guards

Use `#[cfg(not(target_arch = "wasm32"))]` for native-only tests:

```rust
#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn test_functionality() {
        let result = tested_function();
        assert_eq!(result, expected);
    }

    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    fn test_native_only() {
        // Native-only test (uses tokio or filesystem)
    }
}
```

### Test Utilities

Create helper functions in `mod test_utils` within test files for reusable test setup.

### Mock Data

Generate synthetic DICOM data for deterministic tests.

### Performance Assertions

Use `Instant::now()` to measure and assert performance:

```rust
use std::time::Instant;

let start = Instant::now();
// ... do work ...
let elapsed = start.elapsed();
assert!(elapsed.as_millis() < 100, "Operation too slow");
```

### Test Naming Convention

- `tests/*_tests.rs`: Integration tests
- `src/*/mod.rs` → `#[cfg(test)] mod tests { ... }`: Unit tests

## Coordinate Systems

Four coordinate systems are used (see `src/core/coord/`):

1. **World**: 3D world coordinates (millimeters from volume origin)
2. **Screen**: 2D screen coordinates (pixels, origin at top-left)
3. **Voxel**: 3D voxel indices (integer indices into volume array)
4. **Base**: Base coordinate type (uses `glam::Vec3`)

Transformations use `glam::Mat4` (column-major):
```rust
use glam::Mat4;

let transform = Mat4::from_translation(glam::Vec3::new(x, y, z));
```

## Module Organization Principles

- Keep modules focused and single-purpose
- Group related functionality in subdirectories
- Use `mod.rs` to expose public API from subdirectories
- Leverage Rust's module system for clear dependency boundaries

## Feature Flags

Use feature flags for optional functionality:
- `mesh`: 3D mesh capabilities
- `trace-logging`: Detailed diagnostics for debugging

Conditional compilation:
```rust
#[cfg(feature = "mesh")]
mod mesh_renderer;
```

## Documentation Comments

Use `///` for public API documentation:

```rust
/// Converts a CT image series to a 3D volume.
///
/// # Arguments
///
/// * `series` - The image series to convert
///
/// # Returns
///
/// A `CTVolume` containing the converted 3D data
///
/// # Errors
///
/// Returns an error if the series contains incompatible images
pub fn generate_ct_volume(series: &ImageSeries) -> KeplerResult<CTVolume> {
    // ...
}
```

## Performance Considerations

- Use `Arc` for cheap cloning of large resources (Device, Queue)
- Avoid cloning expensive resources (Surface, TextureView)
- Reuse bind groups and buffers when possible
- Be mindful of drop order: textures → buffers → bind groups → pipelines

## Code Quality

- Run `cargo fmt` before committing
- Run `cargo clippy` and address warnings
- Write tests for new functionality
- Keep functions focused and short (< 50 lines when possible)

## Related Documentation

- **Quick Reference**: `QUICK_REFERENCE.md` - Common tasks and commands
- **Architecture**: `ARCHITECTURE.md` - Project structure and modules
- **Build**: `BUILD.md` - Development workflow
- **Rendering**: `RENDERING.md` - GPU and coordinate systems
- **Pitfalls**: `PITFALLS.md` - Common anti-patterns to avoid
