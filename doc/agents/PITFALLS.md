# Common Pitfalls & Anti-Patterns

**Last Updated**: 2025-01-15

## NEVER Do These

### Platform Specificity

❌ **Import `tokio` in WASM-targeted modules**

```rust
// WRONG in WASM-targeted files
use tokio::runtime::Runtime;  // Won't compile

// CORRECT
#[cfg(target_arch = "wasm32")]
use async_lock::Mutex;

#[cfg(not(target_arch = "wasm32"))]
use tokio::sync::Mutex;
```

### Error Handling

❌ **Suppress type errors with `as any`, `@ts-ignore`, `@ts-expect-error`**

```rust
// WRONG
let result: SomeType = data as any;

// CORRECT - Handle the error properly
let result: SomeType = data.parse()?;

// CORRECT - Restrictive casting with proper checks
let result: Option<&SomeType> = data.downcast_ref();
```

❌ **Use empty catch blocks**

```rust
// WRONG
let _ = dangerous_operation().map_err(|e| {
    log::error!("Error: {}", e);
    e
})?;

// CORRECT - Handle or propagate the error
let result = dangerous_operation()?;
```

### GPU Resources

❌ **Clone `wgpu::Device` or `wgpu::Queue`**

They're already wrapped in `Arc` for cheap cloning. Don't double-wrap.

```rust
// WRONG
let device_arc = Arc::new(device);  // Device is already Arc<Device>

// CORRECT
let device_clone = device.clone();  // Cheap Arc clone
```

❌ **Clone `wgpu::Surface` or `wgpu::TextureView`**

These are single-use resources that cannot be cloned.

```rust
// WRONG - Won't compile
let surface2 = surface.clone();

// CORRECT - Use reference or recreate
let surface_view = surface.get_current_texture()?;
```

❌ **Mix OpenGL/WebGL calls with WebGPU**

Use one or the other, never both.

```rust
// WRONG - Mixing APIs
gl::ClearColor(...);
queue.submit(...);

// CORRECT - Stick to WebGPU
let clear_color = wgpu::Color { ... };
render_pass.set_scissor_rect(...);
```

### API Changes

❌ **Forget to update `src/lib.rs` re-exports when changing public API**

```rust
// After adding new public function in application/app.rs
// WRONG - Forget to re-export

// CORRECT - Update lib.rs
// src/lib.rs
pub use application::app::new_public_function;
```

### Code Quality

❌ **Hardcode paths like `C:\share`**

```rust
// WRONG
let path = "C:\\share\\dicom_data";

// CORRECT - Use configurable paths
let path = std::env::var("DICOM_DATA_PATH")
    .unwrap_or_else(|_| "./dicom_data".to_string());
```

❌ **Delete failing tests to "pass"**

```rust
// WRONG
// #[test]
// fn test_broken_feature() { ... }

// CORRECT - Fix the test
#[test]
fn test_broken_feature() {
    assert!(condition, "Should pass after fix");
}
```

## Watch Out For These

### WASM Canvas Sizing

⚠️ **CSS controls canvas size, not Winit**

- Must use `request_inner_size()` to set style width/height
- Canvas element ID must be `wasm-example` for web embedding
- Canvas may appear 0x0 if CSS not configured

```rust
// In WASM initialization
#[cfg(target_arch = "wasm32")]
{
    canvas.set_width(width);
    canvas.set_height(height);
    canvas.style().set_property("width", &format!("{}px", width))?;
    canvas.style().set_property("height", &format!("{}px", height))?;
}
```

### Pipeline Mismatches

⚠️ **Always rebuild pipelines when surface format changes**

```rust
// After resize, recreate pipelines
PassExecutor::update_surface_format(&new_format);

// Recreate view factory after graphics swap
view_factory = DefaultViewFactory::new(graphics.clone());
```

### Texture Format Support

⚠️ **Not all GPUs support `R16Float` filtering**

```rust
// Check before using R16Float
if device_supports_r16float(adapter) {
    use_r16float();
} else {
    // Fall back to Rg8Unorm (packed Hounsfield Units)
    use_rg8unorm();
}
```

### Memory Leaks

⚠️ **Texture pools and buffers must be dropped**

- Use per-frame texture pools (`MeshTexturePool`)
- Avoid long-lived GPU resources when possible
- Explicitly drop unused resources

```rust
// WRONG - Long-lived texture pool
static mut TEXTURE_POOL: TexturePool = ...;

// CORRECT - Drop after use
{
    let pool = TexturePool::new();
    // ... use pool ...
    drop(pool); // Explicit drop
}
```

### Coordinate Transformations

⚠️ **Always use `glam` for math, don't write custom matrix multiplication**

```rust
// WRONG - Custom matrix math
fn multiply_matrices(a: [[f32; 4]; 4], b: [[f32; 4]; 4]) -> [[f32; 4]; 4] {
    // 64 lines of buggy matrix multiplication
}

// CORRECT - Use glam
let result = a * b;
```

### Async in Tests

⚠️ **Tests hang on async operations without proper blocking**

```rust
// WRONG - Test will hang
#[test]
fn test_async() {
    async_function().await;
}

// CORRECT - Use pollster (configured in Cargo.toml)
#[test]
fn test_async() {
    pollster::block_on(async_function());
}
```

### Logging in WASM

⚠️ **Logs not showing in WASM**

- Check browser console (F12)
- Logs are routed to `web_sys::console`
- Ensure logger is initialized in `src/lib.rs`

### Test Platform Guards

⚠️ **Tests often assume native-only behavior**

```rust
// WRONG - Will fail on WASM
#[test]
fn test_filesystem() {
    std::fs::File::open("test.txt")?;
}

// CORRECT - Platform guard
#[test]
#[cfg(not(target_arch = "wasm32"))]
fn test_filesystem() {
    std::fs::File::open("test.txt")?;
}
```

### Spec Delta Changes

⚠️ **Using MODIFIED to add new concern without including previous text**

This causes loss of detail at archive time.

```rust
// WRONG - MODIFIED adds new functionality
## MODIFIED Requirements
### Requirement: User Authentication
User MUST provide MFA token.

// CORRECT - Include full requirement
## MODIFIED Requirements
### Requirement: User Authentication
User MUST provide credentials and MFA token.

#### Scenario: Valid credentials without MFA
- **WHEN** valid credentials provided without MFA
- **THEN** request MFA token

#### Scenario: Valid credentials with MFA
- **WHEN** valid credentials and MFA token provided
- **THEN** authenticate successfully
```

## Common Bugs

### Buffer Size Mismatch

```rust
// WRONG - Buffer size doesn't match data
let buffer = device.create_buffer(&wgpu::BufferDescriptor {
    size: 1024,
    ...
});
queue.write_buffer(&buffer, 0, &data); // data is 2048 bytes

// CORRECT - Match sizes
let buffer = device.create_buffer(&wgpu::BufferDescriptor {
    size: data.len() as u64,
    ...
});
```

### Bind Group Layout Mismatch

```rust
// WRONG - Bind group doesn't match shader
let bind_group = device.create_bind_group(&BindGroupDescriptor {
    layout: &some_layout, // Different from shader expectation
    entries: &[
        BindGroupEntry { binding: 0, resource: ... },
    ],
});
```

### Texture View Format Mismatch

```rust
// WRONG - View format doesn't match texture format
let view = texture.create_view(&TextureViewDescriptor {
    format: Some(wgpu::TextureFormat::Rgba8Unorm), // Texture is R16Float
    ...
});
```

### Pipeline State Not Updated

```rust
// WRONG - Changing settings without recreating pipeline
pass.set_pipeline(&old_pipeline);
// Changed uniforms that require different pipeline
pass.draw(...);

// CORRECT - Recreate pipeline when settings change
pipeline = create_pipeline_with_new_settings();
pass.set_pipeline(&pipeline);
```

## Performance Anti-Patterns

### Excessive Allocations

```rust
// WRONG - New allocation per frame
fn render_frame() {
    for i in 0..1000 {
        let temp = Vec::with_capacity(100); // Allocation per iteration
        // ... use temp ...
    }
}

// CORRECT - Reuse allocation
fn render_frame() {
    let mut temp = Vec::with_capacity(100);
    for i in 0..1000 {
        temp.clear();
        // ... use temp ...
    }
}
```

### Unnecessary GPU Synchronization

```rust
// WRONG - Synchronize on every operation
queue.submit(&[encoder.finish()]);
// ... more work ...
queue.submit(&[encoder2.finish()]);

// CORRECT - Batch operations
encoder.push_debug_group("batch");
// ... multiple operations ...
encoder.pop_debug_group();
queue.submit(&[encoder.finish()]);
```

### Texture Upload Overhead

```rust
// WRONG - Upload texture in many small chunks
for y in 0..height {
    queue.write_texture(
        &ImageCopyTexture { ... },
        &data[y * width..(y + 1) * width],
        &ImageDataLayout { ... },
        Extent3d { width, height: 1, depth_or_array_layers: 1 },
    );
}

// CORRECT - Upload entire texture at once
queue.write_texture(
    &ImageCopyTexture { ... },
    &data,
    &ImageDataLayout { ... },
    Extent3d { width, height, depth_or_array_layers: 1 },
);
```

## Code Review Checklist

Before submitting code, check:

- [ ] No `as any` or type error suppression
- [ ] No tokio in WASM modules
- [ ] Public API changes reflected in `src/lib.rs`
- [ ] No hardcoded paths
- [ ] No mixing OpenGL/WebGL with WebGPU
- [ ] Tests have platform guards for native-only code
- [ ] Buffer sizes match data sizes
- [ ] Bind group layouts match shader expectations
- [ ] Pipelines recreated after surface format changes
- [ ] GPU resources properly dropped

## Related Documentation

- **Quick Reference**: `QUICK_REFERENCE.md` - Common patterns at a glance
- **Conventions**: `CONVENTIONS.md` - Correct patterns
- **Rendering**: `RENDERING.md` - GPU best practices
- **Architecture**: `ARCHITECTURE.md` - System design
