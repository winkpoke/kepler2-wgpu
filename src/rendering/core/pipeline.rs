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
use std::sync::Arc;
use wgpu::util::DeviceExt;

/// Global storage for the swapchain/surface color target format used by onscreen render passes.
///
/// Notes
/// - Some pipelines depend on the target color format (e.g., blending and write mask behavior).
/// - Storing it here avoids threading a format parameter throughout the call graph when a single
///   surface is used. On multi-surface setups, prefer passing the format explicitly.
static SWAPCHAIN_FORMAT: OnceCell<wgpu::TextureFormat> = OnceCell::new();

/// Sets the global swapchain/surface color target format. Safe to call multiple times; subsequent calls will be ignored.
///
/// Parameters
/// - `fmt`: The `wgpu::TextureFormat` used by the surface/swapchain color attachment.
///
/// Notes
/// - Uses `OnceCell` so the first successful set wins; later attempts are no-ops.
/// - Prefer explicit formats for multi-window/surface scenarios instead of relying on global state.
pub fn set_swapchain_format(fmt: wgpu::TextureFormat) {
    // No-op if already set; `OnceCell` ensures thread-safe one-time initialization.
    let _ = SWAPCHAIN_FORMAT.set(fmt);
}

/// Gets the globally stored swapchain/surface color target format, if set.
///
/// Returns
/// - `Option<wgpu::TextureFormat>`: `Some(format)` if previously set, otherwise `None`.
pub fn get_swapchain_format() -> Option<wgpu::TextureFormat> {
    // Surfaces may negotiate formats with the backend; ensure this is set during initialization
    // after surface configuration. Call sites can fall back to reasonable defaults when `None`.
    SWAPCHAIN_FORMAT.get().copied()
}

/// Returns the default depth texture format for mesh rendering across native and WebGPU.
///
/// Rationale
/// - Depth24Plus is widely supported and does not include a stencil component, which we do not need.
/// - This choice avoids backend-specific formats and ensures portability across platforms (Windows, macOS, Linux, WASM).
pub fn get_mesh_depth_format() -> wgpu::TextureFormat {
    wgpu::TextureFormat::Depth24Plus
}

// Pipeline creation is now handled directly without caching for simplified architecture.

/// Creates a texture-quad pipeline for 2D MPR views.
///
/// Parameters
/// - `device`: Logical device used for pipeline creation.
/// - `bind_group_layouts`: Trio of bind group layouts in order [texture, vertex uniforms, fragment uniforms].
/// - `vertex_buffers`: Vertex buffer layouts describing the quad vertex input.
/// - `target_format`: Color target format; typically the swapchain/surface format.
///
/// Returns
/// - `Arc<wgpu::RenderPipeline>`: Shared reference to the newly created pipeline.
pub fn get_or_create_texture_quad_pipeline(
    device: &wgpu::Device,
    bind_group_layouts: [&wgpu::BindGroupLayout; 3],
    vertex_buffers: &[wgpu::VertexBufferLayout<'static>],
    target_format: wgpu::TextureFormat,
) -> Arc<wgpu::RenderPipeline> {
    log::trace!(
        "Creating texture quad pipeline for target format: {:?}",
        target_format
    );
    let pipeline = crate::rendering::view::mpr::mpr_render_context::create_texture_quad_pipeline(
        device,
        bind_group_layouts,
        vertex_buffers,
        target_format,
    );
    Arc::new(pipeline)
}

/// Creates a MIP (Maximum Intensity Projection) render pipeline.
///
/// Parameters
/// - `device`: Device used to create shader modules, layouts, and pipelines.
/// - `bind_group_layout`: Bind group layout for MIP uniforms.
/// - `target_format`: Color target format for the render pass.
///
/// Returns
/// - `wgpu::RenderPipeline`: Newly created MIP pipeline.
pub fn create_mip_pipeline_(
    device: &wgpu::Device,
    bind_group_layout: &wgpu::BindGroupLayout,
    target_format: wgpu::TextureFormat,
) -> wgpu::RenderPipeline {
    // Load MIP shader with vertex and fragment entry points
    let shader = device.create_shader_module(wgpu::include_wgsl!("../shaders/mip.wgsl"));

    // Create pipeline layout with MIP bind group
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("MIP Pipeline Layout"),
        bind_group_layouts: &[bind_group_layout],
        push_constant_ranges: &[],
    });

    // Create the MIP pipeline
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("MIP Pipeline"),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            buffers: &[], // Fullscreen quad generated in vertex shader
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs_main"),
            targets: &[Some(wgpu::ColorTargetState {
                format: target_format,
                blend: Some(wgpu::BlendState::REPLACE),
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: None,
            unclipped_depth: false,
            polygon_mode: wgpu::PolygonMode::Fill,
            conservative: false,
        },
        depth_stencil: None, // No depth for MIP rendering
        multisample: wgpu::MultisampleState {
            count: 1,
            mask: !0,
            alpha_to_coverage_enabled: false,
        },
        multiview: None,
        cache: None,
    })
}

/// Creates the texture-quad render pipeline used by 2D MPR views.
/// Expects three bind group layouts: texture, vertex uniforms, fragment uniforms.
///
/// Parameters
/// - `device`: Device used to create shader modules, layouts, and pipelines.
/// - `bind_group_layouts`: Array of [texture, vertex uniform, fragment uniform] layouts used by the shader.
/// - `vertex_buffers`: Quad vertex buffer layouts.
/// - `target_format`: Color target format for the render pass (swapchain/surface format).
///
/// Returns
/// - `wgpu::RenderPipeline`: Newly created pipeline. Callers should manage pipeline lifecycle as needed.
///
/// Notes
/// - Pipeline state is fully specified for deterministic behavior across platforms.
/// - Blend state is set to REPLACE; adjust if alpha blending is desired.
/// - No depth-stencil state since 2D slice rendering uses separate passes without depth.
///
/// TODO
/// - Parameterize MSAA (`multisample.count`) and culling for performance/quality trade-offs.

/// Creates a basic mesh pipeline with depth testing enabled.
/// Uses global swapchain format if set; otherwise falls back to Rgba8Unorm.
///
/// Parameters
/// - `device`: Logical device used to build shader modules and pipelines.
///
/// Returns
/// - `Arc<wgpu::RenderPipeline>`: Shared pipeline handle for mesh rendering.
///
/// Notes
/// - This pipeline has no bind groups and renders points using `MeshVertex::desc()` layout.
/// - Uses REPLACE blending; adjust if you need alpha.
pub fn get_or_create_mesh_pipeline(device: &wgpu::Device) -> Arc<wgpu::RenderPipeline> {
    get_or_create_mesh_pipeline_with_depth(device, true)
}

/// Creates a basic mesh pipeline with configurable depth testing.
/// This allows creating pipelines with or without depth-stencil state based on render pass requirements.
///
/// Parameters
/// - `device`: Logical device used to build shader modules and pipelines.
/// - `use_depth`: Whether to enable depth testing and depth buffer writes.
///
/// Returns
/// - `Arc<wgpu::RenderPipeline>`: Shared pipeline handle for mesh rendering.
pub fn get_or_create_mesh_pipeline_with_depth(
    device: &wgpu::Device,
    use_depth: bool,
) -> Arc<wgpu::RenderPipeline> {
    log::trace!("Creating mesh pipeline with depth: {}", use_depth);

    // Get target format and topology
    let target_format = get_swapchain_format().unwrap_or(wgpu::TextureFormat::Rgba8Unorm);
    let topology = wgpu::PrimitiveTopology::TriangleList;

    // Mesh shader with both vertex and fragment stages.
    let shader = device.create_shader_module(wgpu::include_wgsl!("../shaders/mesh.wgsl"));

    // Create bind group layouts for uniform buffers
    // Bind group 0: Camera uniforms (view, projection matrices, camera position)
    let camera_bind_group_layout =
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Camera Bind Group Layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

    // Bind group 1: Lighting uniforms (light position, color, material properties)
    let lighting_bind_group_layout =
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Lighting Bind Group Layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

    // Bind group 2: Model uniforms (model matrix, normal matrix)
    let model_bind_group_layout =
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Model Bind Group Layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

    // Bind group 3: Material uniforms (albedo, metallic, roughness, etc.)
    let material_bind_group_layout =
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Material Bind Group Layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

    // Create pipeline layout with uniform buffer bind groups
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Mesh Pipeline Layout"),
        bind_group_layouts: &[
            &camera_bind_group_layout,
            &lighting_bind_group_layout,
            &model_bind_group_layout,
            &material_bind_group_layout,
        ],
        push_constant_ranges: &[],
    });
    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Mesh Pipeline"),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            buffers: &[crate::rendering::mesh::mesh::MeshVertex::desc()],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs_main"),
            targets: &[Some(wgpu::ColorTargetState {
                format: target_format,
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        }),
        primitive: wgpu::PrimitiveState {
            topology,
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: None,
            polygon_mode: wgpu::PolygonMode::Fill,
            unclipped_depth: false,
            conservative: false,
        },
        depth_stencil: if use_depth {
            Some(wgpu::DepthStencilState {
                format: get_mesh_depth_format(),
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            })
        } else {
            None
        },
        multisample: wgpu::MultisampleState {
            count: 1,
            mask: !0,
            alpha_to_coverage_enabled: false,
        },
        multiview: None,
        cache: None,
    });

    Arc::new(pipeline)
}

/// Computes a deterministic signature string for the provided vertex buffer layouts.
///
/// Parameters
/// - `layouts`: Slice of vertex buffer layouts used by a pipeline.
///
/// Returns
/// - `String`: A stable string encoding key properties of the layouts (stride, step mode, attributes).
///
/// Notes
/// - This is used to strengthen cache keys for TextureQuad by reflecting vertex input configuration.
/// - The format is textual for readability; switching to a hash can reduce memory and speed comparisons.
///
/// TODO
/// - Use a hasher (e.g., `ahash`) to produce a compact signature instead of a string.
pub fn vertex_layout_signature(layouts: &[wgpu::VertexBufferLayout<'static>]) -> String {
    let mut s = String::new();
    s.push_str(&format!("count:{};", layouts.len()));
    for (i, vb) in layouts.iter().enumerate() {
        s.push_str(&format!(
            "i:{};stride:{};step:{:?};attrs:{};",
            i,
            vb.array_stride,
            vb.step_mode,
            vb.attributes.len()
        ));
        for a in vb.attributes.iter() {
            s.push_str(&format!(
                "loc:{};off:{};fmt:{:?};",
                a.shader_location, a.offset, a.format
            ));
        }
    }
    s
}

/// Returns a static signature representing the default trio of bind group layouts
/// used by RenderContext for texture rendering.
///
/// Returns
/// - `String`: Signature used as part of cache keys for the texture quad pipeline.
pub fn default_slice_bgl_signature() -> String {
    "texture+vertex+fragment_uniforms_v1".to_string()
}

/// Creates a uniform buffer and a bind group for a given layout and data.
///
/// Parameters
/// - `device`: Logical device used to allocate buffer and bind group.
/// - `layout`: Bind group layout describing the uniform binding at slot 0.
/// - `data`: Uniform POD data to upload.
///
/// Returns
/// - `(wgpu::Buffer, wgpu::BindGroup)`: The created uniform buffer and its bind group.
///
/// Notes
/// - Buffer usage includes UNIFORM and COPY_DST to allow updates via queue.write_buffer.
/// - Binding uses offset 0 and unspecified size (entire buffer).
///
/// TODO
/// - Consider aligning buffer size to `device.limits().min_uniform_buffer_offset_alignment` when using dynamic offsets.
pub fn create_uniform_bind_group<T: bytemuck::Pod>(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    data: &T,
) -> (wgpu::Buffer, wgpu::BindGroup) {
    let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Uniform Buffer"),
        contents: bytemuck::cast_slice(&[*data]),
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    });

    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                buffer: &buffer,
                offset: 0,
                size: None,
            }),
        }],
        label: Some("Uniform Bind Group"),
    });

    (buffer, bind_group)
}

/// Creates vertex-stage uniform bind group and layout.
///
/// Parameters
/// - `device`: Logical device used to allocate resources.
/// - `data`: Vertex-stage uniform POD data.
///
/// Returns
/// - `(wgpu::Buffer, wgpu::BindGroup, wgpu::BindGroupLayout)`: The buffer, bind group, and layout.
///
/// Notes
/// - Layout visibility is restricted to the VERTEX stage.
///
/// TODO
/// - Expose `has_dynamic_offset` and `min_binding_size` for advanced uniform management.
pub fn create_vertex_uniform_bind_group<T: bytemuck::Pod>(
    device: &wgpu::Device,
    data: &T,
) -> (wgpu::Buffer, wgpu::BindGroup, wgpu::BindGroupLayout) {
    let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::VERTEX,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        }],
        label: Some("uniform_bind_group_layout"),
    });

    let (buffer, bind_group) = create_uniform_bind_group(device, &layout, data);
    (buffer, bind_group, layout)
}

/// Creates fragment-stage uniform bind group and layout.
///
/// Parameters
/// - `device`: Logical device used to allocate resources.
/// - `data`: Fragment-stage uniform POD data.
///
/// Returns
/// - `(wgpu::Buffer, wgpu::BindGroup, wgpu::BindGroupLayout)`: The buffer, bind group, and layout.
///
/// Notes
/// - Layout visibility is restricted to the FRAGMENT stage.
///
/// TODO
/// - Expose `has_dynamic_offset` and `min_binding_size` for advanced uniform management.
pub fn create_fragment_uniform_bind_group<T: bytemuck::Pod>(
    device: &wgpu::Device,
    data: &T,
) -> (wgpu::Buffer, wgpu::BindGroup, wgpu::BindGroupLayout) {
    let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        }],
        label: Some("uniform_frag_bind_group_layout"),
    });
    let (buffer, bind_group) = create_uniform_bind_group(device, &layout, data);
    (buffer, bind_group, layout)
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
pub fn create_basic_lighting_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("Basic Lighting Bind Group Layout"),
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        }],
    })
}

/// Function-level comment: Creates a basic mesh pipeline with lighting support using two bind groups.
/// Supports transform uniforms (bind group 0) and lighting uniforms (bind group 1) for basic 3D lighting.
pub fn create_basic_mesh_pipeline_with_lighting(
    device: &wgpu::Device,
    transform_bind_group_layout: &wgpu::BindGroupLayout,
    lighting_bind_group_layout: &wgpu::BindGroupLayout,
    use_depth: bool,
) -> wgpu::RenderPipeline {
    // Use the basic mesh shader with lighting support
    let shader = device.create_shader_module(wgpu::include_wgsl!("../shaders/mesh_basic.wgsl"));

    // Create pipeline layout with two bind groups
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Basic Mesh Pipeline Layout with Lighting"),
        bind_group_layouts: &[transform_bind_group_layout, lighting_bind_group_layout],
        push_constant_ranges: &[],
    });

    // Get target format
    let target_format = get_swapchain_format().unwrap_or(wgpu::TextureFormat::Rgba8Unorm);

    // Create the pipeline
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Basic Mesh Pipeline with Lighting"),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            buffers: &[crate::rendering::mesh::mesh::MeshVertex::desc()],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs_main"),
            targets: &[Some(wgpu::ColorTargetState {
                format: target_format,
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: None, // Temporarily disable culling to test visibility
            unclipped_depth: false,
            polygon_mode: wgpu::PolygonMode::Fill,
            conservative: false,
        },
        depth_stencil: if use_depth {
            Some(wgpu::DepthStencilState {
                format: get_mesh_depth_format(),
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            })
        } else {
            None
        },
        multisample: wgpu::MultisampleState {
            count: 1,
            mask: !0,
            alpha_to_coverage_enabled: false,
        },
        multiview: None,
        cache: None,
    })
}

pub fn create_simple_mesh_pipeline(
    device: &wgpu::Device,
    bind_group_layout: &wgpu::BindGroupLayout,
    use_depth: bool,
) -> wgpu::RenderPipeline {
    // Use the basic mesh shader with minimal uniforms
    let shader = device.create_shader_module(wgpu::include_wgsl!("../shaders/mesh_basic.wgsl"));

    // Create pipeline layout with only one bind group
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Simple Mesh Pipeline Layout"),
        bind_group_layouts: &[bind_group_layout],
        push_constant_ranges: &[],
    });

    // Get target format
    let target_format = get_swapchain_format().unwrap_or(wgpu::TextureFormat::Rgba8Unorm);

    // Create the pipeline
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Simple Mesh Pipeline"),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            buffers: &[crate::rendering::mesh::mesh::MeshVertex::desc()],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs_main"),
            targets: &[Some(wgpu::ColorTargetState {
                format: target_format,
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: None, // Temporarily disable culling to test visibility
            unclipped_depth: false,
            polygon_mode: wgpu::PolygonMode::Fill,
            conservative: false,
        },
        depth_stencil: if use_depth {
            Some(wgpu::DepthStencilState {
                format: get_mesh_depth_format(),
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            })
        } else {
            None
        },
        multisample: wgpu::MultisampleState {
            count: 1,
            mask: !0,
            alpha_to_coverage_enabled: false,
        },
        multiview: None,
        cache: None,
    })
}

// Tests removed - pipeline creation is now handled directly without caching.
#[cfg(test)]
mod tests {
    use super::*;

    /// Function-level comment: 验证全局交换链格式 set/get 的一次性设置行为
    #[test]
    fn test_swapchain_format_set_get() {
        assert!(get_swapchain_format().is_none());
        set_swapchain_format(wgpu::TextureFormat::Bgra8Unorm);
        let fmt = get_swapchain_format().unwrap();
        assert_eq!(fmt, wgpu::TextureFormat::Bgra8Unorm);
        // 再次设置应被忽略
        set_swapchain_format(wgpu::TextureFormat::Rgba8Unorm);
        let fmt2 = get_swapchain_format().unwrap();
        assert_eq!(fmt2, wgpu::TextureFormat::Bgra8Unorm);
    }

    /// Function-level comment: 验证默认深度格式为 Depth24Plus 以跨平台兼容
    #[test]
    fn test_get_mesh_depth_format() {
        assert_eq!(get_mesh_depth_format(), wgpu::TextureFormat::Depth24Plus);
    }
}
