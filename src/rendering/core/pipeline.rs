//! Render pipeline management and utilities for wgpu.
//!
//! This module provides:
//! - A lightweight cache for `wgpu::RenderPipeline` objects keyed by `PipelineKey`.
//! - Helpers to create common pipelines (e.g., texture-quad and mesh) and uniform bind groups.
//! - A global swapchain/surface format accessor to ensure pipelines target the correct color format.
//!
//! Design notes:
//! - Pipeline creation is relatively expensive. Caching pipelines avoids redundant compilation work and
//!   improves frame-time stability, especially in interactive contexts (native and WASM).
//! - Keys aim to be deterministic across runs and platforms. Current signatures use string serialization;
//!   consider migrating to a compact hash for performance.
//! - This module does not manage shader hot-reload; shader modules are compiled at creation time.
//!
//! Invariants:
//! - Bind group layout order for the texture quad pipeline is [texture, vertex uniforms, fragment uniforms].
//! - Vertex buffer layouts used for cache keys must reflect all vertex attributes that influence pipeline creation.
//! - Pipelines created here are valid for the device they are created from and must not be shared across devices.
//!
//! TODO:
//! - Replace string-based signatures with a stable hash for faster keying.
//! - Add depth-stencil configuration options for pipelines that require ordering or testing.
//! - Introduce feature flags for toggling debug trace logs and pipeline instrumentation.
//! - Consider adding shader module caching or a simple hot-reload mechanism in dev builds.
//! - Add tests covering cache hit/miss behavior and signature correctness.
#![allow(dead_code)]

use once_cell::sync::OnceCell;
use std::num::NonZero;
use wgpu::*;

/// Global storage for the swapchain/surface color target format used by onscreen render passes.
///
/// Notes
/// - Some pipelines depend on the target color format (e.g., blending and write mask behavior).
/// - Storing it here avoids threading a format parameter throughout the call graph when a single
///   surface is used. On multi-surface setups, prefer passing the format explicitly.
static SWAPCHAIN_FORMAT: OnceCell<TextureFormat> = OnceCell::new();

/// Sets the global swapchain/surface color target format. Safe to call multiple times; subsequent calls will be ignored.
///
/// Parameters
/// - `fmt`: The `wgpu::TextureFormat` used by the surface/swapchain color attachment.
///
/// Notes
/// - Uses `OnceCell` so the first successful set wins; later attempts are no-ops.
/// - Prefer explicit formats for multi-window/surface scenarios instead of relying on global state.
pub fn set_swapchain_format(fmt: TextureFormat) {
    // No-op if already set; `OnceCell` ensures thread-safe one-time initialization.
    let _ = SWAPCHAIN_FORMAT.set(fmt);
}

/// Gets the globally stored swapchain/surface color target format, if set.
///
/// Returns
/// - `Option<wgpu::TextureFormat>`: `Some(format)` if previously set, otherwise `None`.
pub fn get_swapchain_format() -> Option<TextureFormat> {
    // Surfaces may negotiate formats with the backend; ensure this is set during initialization
    // after surface configuration. Call sites can fall back to reasonable defaults when `None`.
    SWAPCHAIN_FORMAT.get().copied()
}

/// Returns the default depth texture format for mesh rendering across native and WebGPU.
///
/// Rationale
/// - Depth24Plus is widely supported and does not include a stencil component, which we do not need.
/// - This choice avoids backend-specific formats and ensures portability across platforms (Windows, macOS, Linux, WASM).
pub fn get_mesh_depth_format() -> TextureFormat {
    TextureFormat::Depth24Plus
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VolumeRenderMode {
    Mpr,
    Mip,
    Volume,
}

pub struct VolumePipelines {
    pub mpr: RenderPipeline,
    pub mip: RenderPipeline,
    pub volume: RenderPipeline,
}

/// Creates a simple mesh pipeline for BasicMeshContext that only requires one bind group.
/// This is a simplified version that uses the basic_mesh.wgsl shader with minimal uniforms.
///
/// Parameters
/// - `device`: Logical device used to build shader modules and pipelines.
/// - `bind_group_layout`: Single bind group layout for basic uniforms.
/// - `use_depth`: Whether to enable depth testing and depth buffer writes.
///
/// Returns
/// - `wgpu::RenderPipeline`: Newly created simple pipeline.
/// Function-level comment: Creates a bind group layout for basic lighting uniforms.
/// Supports a single directional light with ambient lighting for basic mesh rendering.
pub fn create_basic_lighting_bind_group_layout(device: &Device) -> BindGroupLayout {
    device.create_bind_group_layout(&BindGroupLayoutDescriptor {
        label: Some("Basic Lighting Bind Group Layout"),
        entries: &[BindGroupLayoutEntry {
            binding: 0,
            visibility: ShaderStages::FRAGMENT,
            ty: BindingType::Buffer {
                ty: BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        }],
    })
}

pub fn create_texture_bind_group_layout(device: &Device) -> BindGroupLayout {
    device.create_bind_group_layout(&BindGroupLayoutDescriptor {
        label: Some("Texture Bind Group Layout"),
        entries: &[
            // Texture binding
            BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::FRAGMENT,
                ty: BindingType::Texture {
                    multisampled: false,
                    view_dimension: TextureViewDimension::D3,
                    sample_type: TextureSampleType::Float { filterable: true },
                },
                count: None,
            },

            // Sampler binding
            BindGroupLayoutEntry {
                binding: 1,
                visibility: ShaderStages::FRAGMENT,
                ty: BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            },
        ],
    })
}

// Create bind group layout for uniforms (group 1)
pub fn create_uniform_bind_group_layout(device: &Device, min_binding_size: Option<NonZero<u64>>) -> BindGroupLayout {
    device.create_bind_group_layout(&BindGroupLayoutDescriptor {
        label: Some("Uniform Bind Group Layout"),
        entries: &[BindGroupLayoutEntry {
            binding: 0,
            visibility: ShaderStages::FRAGMENT,
            ty: BindingType::Buffer {
                ty: BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: min_binding_size,
            },
            count: None,
        }],
    })
}

/// Function-level comment: Creates a basic mesh pipeline with lighting support using two bind groups.
/// Supports transform uniforms (bind group 0) and lighting uniforms (bind group 1) for basic 3D lighting.
pub fn create_basic_mesh_pipeline_with_lighting(
    device: &Device,
    transform_bind_group_layout: &BindGroupLayout,
    lighting_bind_group_layout: &BindGroupLayout,
    use_depth: bool,
) -> RenderPipeline {
    // Use the basic mesh shader with lighting support
    let shader = device.create_shader_module(wgpu::include_wgsl!("../shaders/mesh_basic.wgsl"));

    // Create pipeline layout with two bind groups
    let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
        label: Some("Basic Mesh Pipeline Layout with Lighting"),
        bind_group_layouts: &[transform_bind_group_layout, lighting_bind_group_layout],
        push_constant_ranges: &[],
    });

    // Get target format
    let target_format = get_swapchain_format().unwrap_or(TextureFormat::Rgba8Unorm);

    // Create the pipeline
    device.create_render_pipeline(&RenderPipelineDescriptor {
        label: Some("Basic Mesh Pipeline with Lighting"),
        layout: Some(&pipeline_layout),
        vertex: VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            buffers: &[crate::rendering::mesh::mesh::MeshVertex::desc()],
            compilation_options: PipelineCompilationOptions::default(),
        },
        fragment: Some(FragmentState {
            module: &shader,
            entry_point: Some("fs_main"),
            targets: &[Some(ColorTargetState {
                format: target_format,
                blend: Some(BlendState::ALPHA_BLENDING),
                write_mask: ColorWrites::ALL,
            })],
            compilation_options: PipelineCompilationOptions::default(),
        }),
        primitive: PrimitiveState {
            topology: PrimitiveTopology::TriangleList,
            strip_index_format: None,
            front_face: FrontFace::Ccw,
            cull_mode: None, // Temporarily disable culling to test visibility
            unclipped_depth: false,
            polygon_mode: PolygonMode::Fill,
            conservative: false,
        },
        depth_stencil: if use_depth {
            Some(DepthStencilState {
                format: get_mesh_depth_format(),
                depth_write_enabled: true,
                depth_compare: CompareFunction::Less,
                stencil: StencilState::default(),
                bias: DepthBiasState::default(),
            })
        } else {
            None
        },
        multisample: MultisampleState {
            count: 1,
            mask: !0,
            alpha_to_coverage_enabled: false,
        },
        multiview: None,
        cache: None,
    })
}

// Function-level comment: Creates a volume pipeline for rendering 3D volumes.
pub fn create_volume_pipeline(
    device: &Device,
    target_format: TextureFormat,
    texture_bind_group_layout: &BindGroupLayout,
    uniform_bind_group_layout: &BindGroupLayout,
) -> RenderPipeline {
    // Use the volume shader
    let shader = device.create_shader_module(ShaderModuleDescriptor {
        label: Some("Volume Shader"),
        source: ShaderSource::Wgsl(
            include_str!("../shaders/mesh_volume.wgsl").into(),
        ),
    });

    // Create pipeline layout with two bind groups
    let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
        label: Some("Volume Pipeline Layout"),
        bind_group_layouts: &[texture_bind_group_layout, uniform_bind_group_layout],
        push_constant_ranges: &[],
    });

    // Create the pipeline
    device.create_render_pipeline(&RenderPipelineDescriptor {
        label: Some("Volume Pipeline"),
        layout: Some(&pipeline_layout),
        vertex: VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            buffers: &[],
            compilation_options: PipelineCompilationOptions::default(),
        },
        fragment: Some(FragmentState {
            module: &shader,
            entry_point: Some("fs_main"),
            targets: &[Some(ColorTargetState {
                format: target_format,
                blend: Some(BlendState::ALPHA_BLENDING),
                write_mask: ColorWrites::ALL,
            })],
            compilation_options: PipelineCompilationOptions::default(),
        }),
        primitive: PrimitiveState {
            topology: PrimitiveTopology::TriangleStrip,
            strip_index_format: None,
            front_face: FrontFace::Ccw,
            cull_mode: None,
            unclipped_depth: false,
            polygon_mode: PolygonMode::Fill,
            conservative: false,
        },
        depth_stencil: Some(DepthStencilState {
            format: get_mesh_depth_format(),
            depth_write_enabled: false,
            depth_compare: CompareFunction::Always,
            stencil: StencilState::default(),
            bias: DepthBiasState::default(),
        }),
        multisample: MultisampleState {
            count: 1,
            mask: !0,
            alpha_to_coverage_enabled: false,
        },
        multiview: None,
        cache: None,
    })
}

pub fn create_mip_pipeline(
    device: &Device,
    target_format: TextureFormat,
    texture_bind_group_layout: &BindGroupLayout,
    uniform_bind_group_layout: &BindGroupLayout,
) -> RenderPipeline {
    // Use the volume shader
    let shader = device.create_shader_module(ShaderModuleDescriptor {
        label: Some("MIP Shader"),
        source: ShaderSource::Wgsl(
            include_str!("../shaders/mip.wgsl").into(),
        ),
    });

    // Create pipeline layout with two bind groups
    let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
        label: Some("MIP Pipeline Layout"),
        bind_group_layouts: &[texture_bind_group_layout, uniform_bind_group_layout],
        push_constant_ranges: &[],
    });

    // Create the pipeline
    device.create_render_pipeline(&RenderPipelineDescriptor {
        label: Some("MIP Render Pipeline"),
        layout: Some(&pipeline_layout),
        vertex: VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            buffers: &[], // fullscreen quad procedural
            compilation_options: PipelineCompilationOptions::default(),
        },
        fragment: Some(FragmentState {
            module: &shader,
            entry_point: Some("fs_main"),
            targets: &[Some(ColorTargetState {
                format: target_format,
                blend: Some(BlendState::REPLACE),
                write_mask: ColorWrites::ALL,
            })],
            compilation_options: PipelineCompilationOptions::default(),
        }),
        primitive: PrimitiveState {
            topology: PrimitiveTopology::TriangleStrip,
            strip_index_format: None,
            front_face: FrontFace::Ccw,
            cull_mode: None,
            unclipped_depth: false,
            polygon_mode: PolygonMode::Fill,
            conservative: false,
        },
        depth_stencil: None,
        multisample: MultisampleState {
            count: 1,
            mask: !0,
            alpha_to_coverage_enabled: false,
        },
        multiview: None,
        cache: None,
    })
}

/// Creates a texture quad pipeline for MPR rendering.
/// This pipeline is specifically designed for rendering 2D texture quads in medical imaging contexts.
pub fn create_texture_quad_pipeline(
    device: &Device,
    bind_group_layouts: [&BindGroupLayout; 3],
    vertex_buffers: &[VertexBufferLayout<'static>],
    target_format: TextureFormat,
) -> RenderPipeline {
    // Single shader module with both vertex and fragment entry points.
    let shader = device.create_shader_module(wgpu::include_wgsl!("../shaders/shader_tex.wgsl"));

    // Pipeline layout defines bind group layout order; must match shader binding expectations.
    let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
        label: Some("MPR Pipeline Layout"),
        bind_group_layouts: &bind_group_layouts,
        push_constant_ranges: &[],
    });

    // Full pipeline descriptor. All fields annotated for clarity.
    device.create_render_pipeline(&RenderPipelineDescriptor {
        label: Some("MPR Pipeline"),
        layout: Some(&pipeline_layout),
        vertex: VertexState {
            module: &shader,
            entry_point: Some("vs_main"), // WGSL entry point for vertex stage
            buffers: vertex_buffers,      // Vertex buffer layouts (position, texcoord, etc.)
            compilation_options: PipelineCompilationOptions::default(),
        },
        fragment: Some(FragmentState {
            module: &shader,
            entry_point: Some("fs_main"), // WGSL entry point for fragment stage
            targets: &[Some(ColorTargetState {
                format: target_format, // Target color format (swapchain surface)
                blend: Some(wgpu::BlendState::REPLACE), // No blending; write replaces previous value
                write_mask: wgpu::ColorWrites::ALL,     // Write all color channels
            })],
            compilation_options: PipelineCompilationOptions::default(),
        }),
        primitive: PrimitiveState {
            topology: PrimitiveTopology::TriangleList, // Quad rendered as two triangles
            strip_index_format: None,
            front_face: FrontFace::Ccw,
            cull_mode: None, // No face culling; adjust for performance if needed
            unclipped_depth: false,
            polygon_mode: PolygonMode::Fill,
            conservative: false,
        },
        depth_stencil: None, // No depth testing for 2D slice rendering
        multisample: MultisampleState {
            count: 1, // No MSAA; parameterize for quality improvements
            mask: !0,
            alpha_to_coverage_enabled: false,
        },
        multiview: None,
        cache: None,
    })
}

/// Creates volume rendering pipelines.
/// This function is responsible for setting up the MPR, MIP, and volume pipelines.
/// It is a central component of the volume rendering pipeline.
pub fn create_volume_pipelines(
    device: &Device,
    target_format: TextureFormat,
    texture_layout: &BindGroupLayout,
    volume_uniform_layout: &BindGroupLayout,
    mpr_layouts: [&BindGroupLayout; 3],
    vertex_buffers: &[VertexBufferLayout<'static>],
) -> VolumePipelines {
    VolumePipelines {
        mpr: create_texture_quad_pipeline(
            device,
            mpr_layouts,
            vertex_buffers,
            target_format,
        ),
        mip: create_mip_pipeline(
            device,
            target_format,
            texture_layout,
            volume_uniform_layout,
        ),
        volume: create_volume_pipeline(
            device,
            target_format,
            texture_layout,
            volume_uniform_layout,
        ),
    }
}

/// Selects a volume pipeline based on the render mode.
/// This function is used to switch between MPR, MIP, and volume pipelines.
/// It is a key component of the volume rendering pipeline.
pub fn select_pipeline<'a>(
    pipelines: &'a VolumePipelines,
    mode: VolumeRenderMode,
) -> &'a RenderPipeline {
    match mode {
        VolumeRenderMode::Mpr => &pipelines.mpr,
        VolumeRenderMode::Mip => &pipelines.mip,
        VolumeRenderMode::Volume => &pipelines.volume,
    }
}

// Tests removed - pipeline creation is now handled directly without caching.
#[cfg(test)]
mod tests {
    use super::*;

    /// Test the swapchain format set/get behavior.
    #[test]
    fn test_swapchain_format_set_get() {
        assert!(get_swapchain_format().is_none());
        set_swapchain_format(wgpu::TextureFormat::Bgra8Unorm);
        let fmt = get_swapchain_format().unwrap();
        assert_eq!(fmt, wgpu::TextureFormat::Bgra8Unorm);
        // Test setting a different format
        set_swapchain_format(wgpu::TextureFormat::Rgba8Unorm);
        let fmt2 = get_swapchain_format().unwrap();
        assert_eq!(fmt2, wgpu::TextureFormat::Bgra8Unorm);
    }

    /// Test the default depth format for volume rendering.
    /// This format is used for depth testing in 2D slice rendering.
    /// It is a common choice for cross-platform compatibility.
    #[test]
    fn test_get_mesh_depth_format() {
        assert_eq!(get_mesh_depth_format(), wgpu::TextureFormat::Depth24Plus);
    }
}