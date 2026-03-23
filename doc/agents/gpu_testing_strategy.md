# GPU Testing Strategy

## Overview

GPU code in Kepler2-WGPU (WebGPU/wgpu-native) presents unique testing challenges due to:
- Hardware-dependent behavior (different GPUs, drivers)
- Limited mock implementations for WebGPU
- Native-only code (cannot test in WASM browsers)
- GPU memory and state cannot be directly inspected

## Strategy: Hybrid Approach

### Primary Strategy: Offline Testing (Logic without GPU)

**Rationale**:
- Test pipeline creation logic, bind group validation, format compatibility without requiring actual GPU
- Test texture upload logic (bounds checking, format validation) without GPU memory allocation
- Test shader compilation error handling without executing shaders
- Enables CI testing on all platforms without GPU requirements

### Secondary Strategy: Native GPU Testing (Hardware Required)

**Rationale**:
- Validate actual GPU resource creation and cleanup
- Test rendering output visual correctness (snapshot testing)
- Detect GPU-specific bugs (driver issues, memory limits)

### Tertiary Strategy: Mock Testing (Limited Use)

**Rationale**:
- Mock GPU device for unit tests where offline testing insufficient
- Limitations: Cannot test actual rendering, memory layout, driver behavior
- Use only for logic tests that don't require GPU execution

## Offline GPU Testing

### Test File: `tests/gpu_offline_tests.rs`

### 1. Pipeline Creation Logic Tests

```rust
#[cfg(test)]
mod pipeline_tests {
    use crate::rendering::pipeline::PipelineManager;

    #[test]
    fn test_pipeline_layout_validation() {
        // Test that bind group layout matches shader reflection
        let layout = PipelineManager::create_bind_group_layout();
        let expected_bindings = vec![
            BindGroupEntry { binding: 0, visibility: ShaderStage::VERTEX },
            BindGroupEntry { binding: 1, visibility: ShaderStage::FRAGMENT },
        ];

        assert_eq!(layout.entries, expected_bindings);
    }

    #[test]
    fn test_pipeline_shader_stage_validation() {
        // Test that shader stages are correctly classified
        let shader = r#"
            @vertex
            fn vs_main() -> @builtin(position) vec4<f32> {
                return vec4<f32>(0.0, 0.0, 0.0, 1.0);
            }

            @fragment
            fn fs_main() -> @location(0) vec4<f32> {
                return vec4<f32>(1.0, 1.0, 1.0, 1.0);
            }
        "#;

        let stages = PipelineManager::parse_shader_stages(shader);
        assert_eq!(stages.len(), 2);
        assert!(stages.contains(&ShaderStage::VERTEX));
        assert!(stages.contains(&ShaderStage::FRAGMENT));
    }

    #[test]
    fn test_pipeline_recreation_on_format_change() {
        // Test that pipeline is recreated when surface format changes
        let mut manager = PipelineManager::new();
        let format1 = TextureFormat::Bgra8UnormSrgb;
        let format2 = TextureFormat::Rgba8UnormSrgb;

        manager.create_pipeline(format1);
        manager.create_pipeline(format2);

        // Verify that pipeline was recreated (different handle or hash)
        assert_ne!(manager.get_pipeline(format1), manager.get_pipeline(format2));
    }
}
```

### 2. Bind Group Layout Tests

```rust
#[cfg(test)]
mod bind_group_tests {
    use crate::rendering::pipeline::BindGroupLayout;

    #[test]
    fn test_bind_group_binding_count_validation() {
        // Test that bind group has correct number of bindings
        let layout = BindGroupLayout::new()
            .add_binding(0, BindingType::UniformBuffer)
            .add_binding(1, BindingType::SampledTexture)
            .build();

        assert_eq!(layout.binding_count(), 2);
    }

    #[test]
    fn test_bind_group_visibility_validation() {
        // Test that binding visibility matches shader stages
        let layout = BindGroupLayout::new()
            .add_binding(0, BindingType::UniformBuffer, ShaderStage::VERTEX | ShaderStage::FRAGMENT)
            .build();

        let vertex_bindings = layout.get_bindings_for_stage(ShaderStage::VERTEX);
        let fragment_bindings = layout.get_bindings_for_stage(ShaderStage::FRAGMENT);

        assert_eq!(vertex_bindings.len(), 1);
        assert_eq!(fragment_bindings.len(), 1);
    }

    #[test]
    fn test_bind_group_invalid_binding_rejected() {
        // Test that invalid binding indices are rejected
        let result = BindGroupLayout::new()
            .add_binding(16, BindingType::UniformBuffer) // Exceeds max binding count (usually 4-16)
            .build();

        assert!(result.is_err());
        assert!(matches!(result, Err(PipelineError::InvalidBindingIndex(_))));
    }
}
```

### 3. Texture Format Compatibility Tests

```rust
#[cfg(test)]
mod texture_format_tests {
    use crate::rendering::texture::Texture;
    use wgpu::TextureFormat;

    #[test]
    fn test_texture_format_supported() {
        // Test that texture formats are supported
        let supported_formats = vec![
            TextureFormat::R8Unorm,
            TextureFormat::R16Snorm,
            TextureFormat::Rgba8UnormSrgb,
            TextureFormat::Bgra8UnormSrgb,
        ];

        for format in supported_formats {
            assert!(Texture::is_format_supported(format), "Format {:?} not supported", format);
        }
    }

    #[test]
    fn test_texture_format_conversion_validation() {
        // Test that format conversions are valid
        let conversion = TextureFormatConversion::new(
            TextureFormat::R16Snorm,  // Input
            TextureFormat::Rgba8UnormSrgb,  // Output
        );

        assert!(conversion.is_valid());
        assert_eq!(conversion.output_channels(), 4); // R16 → RGBA (1 to 4 channels)
    }

    #[test]
    fn test_texture_format_conversion_invalid() {
        // Test that invalid format conversions are rejected
        let conversion = TextureFormatConversion::new(
            TextureFormat::Rgba32Float,  // Input (4 channels, float)
            TextureFormat::R8Unorm,       // Output (1 channel, normalized)
        );

        assert!(!conversion.is_valid());
        assert!(matches!(
            conversion.error(),
            Some(TextureError::UnsupportedFormatConversion)
        ));
    }
}
```

### 4. Shader Reflection Tests

```rust
#[cfg(test)]
mod shader_reflection_tests {
    use crate::rendering::shader::ShaderReflection;

    #[test]
    fn test_shader_uniform_detection() {
        // Test that uniforms are correctly detected from shader
        let shader = r#"
            struct Uniforms {
                transform: mat4x4<f32>,
                color: vec4<f32>,
            }

            @group(0) @binding(0)
            var<uniform> uniforms: Uniforms;

            @vertex
            fn vs_main() -> @builtin(position) vec4<f32> {
                return uniforms.transform * vec4<f32>(0.0, 0.0, 0.0, 1.0);
            }
        "#;

        let reflection = ShaderReflection::parse(shader);
        let uniforms = reflection.get_uniforms();

        assert_eq!(uniforms.len(), 2);
        assert!(uniforms.iter().any(|u| u.name == "transform"));
        assert!(uniforms.iter().any(|u| u.name == "color"));
    }

    #[test]
    fn test_shader_texture_binding_detection() {
        // Test that texture bindings are correctly detected
        let shader = r#"
            @group(0) @binding(1)
            var tex: texture_2d<f32>;

            @fragment
            fn fs_main() -> @location(0) vec4<f32> {
                return textureLoad(tex, vec2<i32>(0, 0));
            }
        "#;

        let reflection = ShaderReflection::parse(shader);
        let textures = reflection.get_texture_bindings();

        assert_eq!(textures.len(), 1);
        assert_eq!(textures[0].binding, 1);
        assert_eq!(textures[0].sampled, false); // textureLoad, not textureSample
    }
}
```

## Native GPU Testing (Hardware Required)

### Test File: `tests/gpu_native_tests.rs`

```rust
#[cfg(test)]
#[cfg(not(target_arch = "wasm32"))]
mod gpu_native_tests {
    use crate::rendering::core::graphics::GraphicsState;

    #[test]
    fn test_gpu_device_creation() {
        // Test that GPU device can be created
        let graphics = GraphicsState::new();
        assert!(graphics.is_ok());

        let graphics = graphics.unwrap();
        assert!(graphics.device().is_some());
        assert!(graphics.queue().is_some());
    }

    #[test]
    fn test_gpu_surface_format_detection() {
        // Test that surface format is correctly detected
        let graphics = GraphicsState::new().unwrap();
        let format = graphics.surface_format();

        assert!(format.is_some());
        let format = format.unwrap();

        // Verify format is valid for rendering
        assert!(matches!(
            format,
            TextureFormat::Bgra8UnormSrgb | TextureFormat::Rgba8UnormSrgb
        ));
    }

    #[test]
    fn test_gpu_pipeline_creation() {
        // Test that pipeline can be created with actual GPU
        let graphics = GraphicsState::new().unwrap();
        let pipeline = graphics.create_pipeline();

        assert!(pipeline.is_ok());
        let pipeline = pipeline.unwrap();

        // Verify pipeline has valid handle
        assert!(pipeline.handle().is_some());
    }

    #[test]
    fn test_gpu_texture_creation() {
        // Test that texture can be created on actual GPU
        let graphics = GraphicsState::new().unwrap();

        let texture = graphics.create_texture(512, 512, TextureFormat::R16Snorm);
        assert!(texture.is_ok());

        let texture = texture.unwrap();
        assert_eq!(texture.width(), 512);
        assert_eq!(texture.height(), 512);
    }

    #[test]
    fn test_gpu_texture_cleanup() {
        // Test that texture memory is freed when dropped
        use std::alloc::{GlobalAlloc, Layout, System};

        let before_allocations = System.allocated_count();

        {
            let graphics = GraphicsState::new().unwrap();
            let _texture = graphics.create_texture(512, 512, TextureFormat::R16Snorm);
        } // Texture and graphics dropped here

        let after_allocations = System.allocated_count();

        // Verify memory was freed (allowing for GPU driver overhead)
        assert!(after_allocations <= before_allocations + 10); // Some GPU allocations persist (driver cache)
    }
}
```

## GPU Coverage Targets (Adjusted for Testability)

### Original vs Realistic Targets

| File | Original Target | Adjusted Target | Rationale |
|-------|----------------|------------------|------------|
| `graphics.rs` | 50% | 30% | Many functions require actual GPU device |
| `pipeline.rs` | 40% | 25% | Pipeline validation tested offline, creation requires GPU |
| `texture.rs` | 50% | 35% | Format validation tested offline, upload requires GPU |
| `shader.rs` | 45% | 40% | Reflection tested offline, compilation requires GPU |

### Excluded Code from Coverage

**GPU-specific exclusions**:
```rust
#[cfg_attr(coverage, no_coverage)]
pub unsafe fn upload_texture_to_gpu(texture: &Texture, data: &[u8]) -> Result<(), Error> {
    // FFI call to WebGPU - cannot test
    wgpu_native_texture_upload(texture.handle, data.as_ptr(), data.len())
}
```

**Documented exclusions in `doc/agents/coverage_methodology.md`**:
- FFI calls to WebGPU (`wgpu-native` bindings)
- GPU driver allocations (outside Rust control)
- Asynchronous GPU operations (awaited futures)
- GPU synchronization primitives (fences, semaphores)

## Mock GPU Testing (Limited Use)

### When to Use Mocks

1. **Logic tests** that don't require actual GPU execution
2. **Error handling tests** for GPU-specific errors
3. **Resource limit tests** (without requiring actual hardware limits)

### Mock GPU Device

```rust
#[cfg(test)]
mod mock_gpu_tests {
    use crate::rendering::core::graphics::MockGraphicsState;

    #[test]
    fn test_mock_gpu_device_creation() {
        // Test logic with mock device
        let mock_gpu = MockGraphicsState::new();

        let result = mock_gpu.create_pipeline();
        assert!(result.is_ok());
    }

    #[test]
    fn test_mock_gpu_texture_upload_bounds() {
        // Test texture bounds checking with mock
        let mock_gpu = MockGraphicsState::new();

        // Test exceeding max texture size
        let result = mock_gpu.create_texture(1_000_000, 1_000_000, TextureFormat::R8Unorm);
        assert!(result.is_err());
        assert!(matches!(result, Err(GraphicsError::TextureSizeExceeded)));
    }

    #[test]
    fn test_mock_gpu_memory_limit() {
        // Test memory limit enforcement with mock
        let mock_gpu = MockGraphicsState::with_memory_limit(1024 * 1024); // 1MB limit

        let texture1 = mock_gpu.create_texture(512, 512, TextureFormat::R8Unorm);
        assert!(texture1.is_ok()); // 512x512x1 = 262KB

        let texture2 = mock_gpu.create_texture(1024, 1024, TextureFormat::R8Unorm);
        assert!(texture2.is_err()); // 1024x1024x1 = 1MB, exceeds limit (262KB + 1MB > 1MB)
    }
}
```

### Mock Implementation

```rust
// tests/mock_gpu.rs

pub struct MockGraphicsState {
    memory_used: Arc<Mutex<usize>>,
    memory_limit: usize,
    device_handle: Option<u64>,
}

impl MockGraphicsState {
    pub fn new() -> Self {
        Self {
            memory_used: Arc::new(Mutex::new(0)),
            memory_limit: usize::MAX,
            device_handle: Some(0xDEADBEEF),
        }
    }

    pub fn with_memory_limit(limit: usize) -> Self {
        Self {
            memory_used: Arc::new(Mutex::new(0)),
            memory_limit: limit,
            device_handle: Some(0xDEADBEEF),
        }
    }
}

impl GraphicsState for MockGraphicsState {
    fn device(&self) -> Option<&Device> {
        None // Mock has no real device
    }

    fn create_texture(&self, width: u32, height: u32, format: TextureFormat) -> Result<Texture, Error> {
        let size = width as usize * height as usize * format.bytes_per_pixel();
        let mut memory_used = self.memory_used.lock().unwrap();

        if memory_used + size > self.memory_limit {
            return Err(Error::MemoryLimitExceeded);
        }

        *memory_used += size;
        Ok(Texture::mock(width, height, format, self.memory_used.clone()))
    }
}
```

## GPU Testing Limitations

### What Cannot Be Tested

1. **Actual Rendering Output**
   - Cannot verify visual correctness without GPU execution
   - **Workaround**: Snapshot testing with reference images (complex)

2. **GPU Driver Bugs**
   - Different GPUs (NVIDIA, AMD, Intel) have different driver behaviors
   - **Workaround**: Test on multiple hardware in CI (limited)

3. **GPU Memory Layout**
   - Cannot inspect GPU memory directly
   - **Workaround**: Test texture upload bounds, assume driver correctness

4. **Shader Performance**
   - Cannot benchmark shader execution time without GPU
   - **Workaround**: Use performance profiling on actual hardware

5. **Asynchronous GPU Operations**
   - Difficult to test async GPU fences and semaphores
   - **Workaround**: Test synchronization logic, assume driver correctness

## CI Integration

### Offline Tests (All Platforms)

```yaml
# .github/workflows/gpu-tests.yml
name: GPU Tests (Offline)

on: [push, pull_request]

jobs:
  gpu-offline:
    runs-on: ubuntu-latest

    steps:
      - uses: actions/checkout@v3

      - name: Run offline GPU tests
        run: |
          cargo test --test gpu_offline_tests
```

### Native GPU Tests (With GPU)

```yaml
  gpu-native:
    runs-on: ubuntu-latest
    # Note: Requires GPU runner (self-hosted or specific hardware)

    steps:
      - uses: actions/checkout@v3

      - name: Install GPU drivers
        run: |
          sudo apt-get update
          sudo apt-get install -y mesa-vulkan-drivers

      - name: Run native GPU tests
        run: |
          cargo test --test gpu_native_tests
```

## GPU Testing Strategy Document

**File**: `doc/agents/gpu_testing_strategy.md` (this document)

**Purpose**: Document GPU testing approach, limitations, and coverage adjustments

**Maintained by**: Rendering team

**Updated**: When GPU testing tools or WebGPU API changes

## References

- WebGPU specification: https://gpuweb.github.io/gpuweb/
- wgpu-native: https://github.com/gfx-rs/wgpu-native
- GPU testing best practices: https://google.github.io/agi/gpu-testing.html
- Mock testing strategies: https://martinfowler.com/bliki/TestDouble.html
