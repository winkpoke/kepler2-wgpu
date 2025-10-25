#![allow(dead_code)]

use std::sync::Arc;
use wgpu::util::DeviceExt;

/// Global GPU state shared across all MPR views
/// Contains pipeline, bind group layouts, and shared vertex/index buffers
/// This struct represents the "context" level in the MPR architecture
pub struct MprRenderContext {
    /// Shared render pipeline for all MPR views
    pub render_pipeline: Arc<wgpu::RenderPipeline>,
    /// Bind group layout for 3D texture and sampler
    pub texture_bind_group_layout: wgpu::BindGroupLayout,
    
    /// Bind group layout for vertex uniforms
    pub vertex_bind_group_layout: wgpu::BindGroupLayout,
    
    /// Bind group layout for fragment uniforms
    pub fragment_bind_group_layout: wgpu::BindGroupLayout,
    
    /// Shared vertex buffer (quad vertices)
    pub vertex_buffer: wgpu::Buffer,
    
    /// Shared index buffer (quad indices)
    pub index_buffer: wgpu::Buffer,
    
    /// Number of indices in the index buffer
    pub num_indices: u32,
}

#[repr(C)]
#[derive(Copy, Clone, Default, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 3],
    tex_coords: [f32; 2],
}

impl Vertex {
    const ATTRIBS: [wgpu::VertexAttribute; 2] =
        wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x2];

    fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBS,
        }
    }
}

/// Creates a texture quad pipeline for MPR rendering.
/// This pipeline is specifically designed for rendering 2D texture quads in medical imaging contexts.
pub fn create_texture_quad_pipeline(
    device: &wgpu::Device,
    bind_group_layouts: [&wgpu::BindGroupLayout; 3],
    vertex_buffers: &[wgpu::VertexBufferLayout<'static>],
    target_format: wgpu::TextureFormat,
) -> wgpu::RenderPipeline {
    // Single shader module with both vertex and fragment entry points.
    let shader = device.create_shader_module(wgpu::include_wgsl!("../../shaders/shader_tex.wgsl"));
    // Pipeline layout defines bind group layout order; must match shader binding expectations.
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Render Pipeline Layout"),
        bind_group_layouts: &bind_group_layouts,
        push_constant_ranges: &[],
    });

    // Full pipeline descriptor. All fields annotated for clarity.
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Render Pipeline"),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"), // WGSL entry point for vertex stage
            buffers: vertex_buffers,        // Vertex buffer layouts (position, texcoord, etc.)
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs_main"), // WGSL entry point for fragment stage
            targets: &[Some(wgpu::ColorTargetState {
                format: target_format,             // Target color format (swapchain surface)
                blend: Some(wgpu::BlendState::REPLACE), // No blending; write replaces previous value
                write_mask: wgpu::ColorWrites::ALL,     // Write all color channels
            })],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList, // Quad rendered as two triangles
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: None,                    // No face culling; adjust for performance if needed
            polygon_mode: wgpu::PolygonMode::Fill,
            unclipped_depth: false,
            conservative: false,
        },
        depth_stencil: None, // No depth testing for 2D slice rendering
        multisample: wgpu::MultisampleState {
            count: 1,                          // No MSAA; parameterize for quality improvements
            mask: !0,
            alpha_to_coverage_enabled: false,
        },
        multiview: None,
        cache: None,
    })
}

const VERTICES: &[Vertex] = &[
    Vertex {
        position: [-1., 1., 0.0],
        tex_coords: [0.0, 0.0],
    },
    Vertex {
        position: [-1., -1., 0.0],
        tex_coords: [0.0, 1.0],
    },
    Vertex {
        position: [1., -1., 0.0],
        tex_coords: [1.0, 1.0],
    },
    Vertex {
        position: [1., 1., 0.0],
        tex_coords: [1.0, 0.0],
    },
];

const INDICES: &[u16] = &[0, 1, 2, 0, 2, 3];

impl MprRenderContext {
    /// Create a new MprRenderContext with shared GPU resources
    /// This initializes the global state that can be shared across multiple MPR views
    /// 
    /// # Arguments
    /// * `manager` - Pipeline manager for caching render pipelines
    /// * `device` - WGPU device for creating GPU resources
    /// 
    /// # Returns
    /// A new MprRenderContext with initialized shared resources
    pub fn new(
        device: &wgpu::Device,
    ) -> Self {
        // Create bind group layout for 3D texture and sampler
        let texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D3,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
                label: Some("mpr_texture_bind_group_layout"),
            });

        // Create bind group layouts for uniforms
        let vertex_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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
            label: Some("mpr_vertex_uniform_bind_group_layout"),
        });

        let fragment_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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
            label: Some("mpr_fragment_uniform_bind_group_layout"),
        });

        // Get target format for pipeline creation
        let target_format = crate::rendering::core::pipeline::get_swapchain_format()
            .unwrap_or(wgpu::TextureFormat::Rgba8Unorm);

        // Create render pipeline directly
        let bgls: [&wgpu::BindGroupLayout; 3] = [
            &texture_bind_group_layout,
            &vertex_bind_group_layout,
            &fragment_bind_group_layout,
        ];
        let render_pipeline = Arc::new(create_texture_quad_pipeline(
            device,
            bgls,
            &[Vertex::desc()],
            target_format,
        ));

        // Create shared vertex buffer
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("MPR Shared Vertex Buffer"),
            contents: bytemuck::cast_slice(VERTICES),
            usage: wgpu::BufferUsages::VERTEX,
        });

        // Create shared index buffer
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("MPR Shared Index Buffer"),
            contents: bytemuck::cast_slice(INDICES),
            usage: wgpu::BufferUsages::INDEX,
        });
        let num_indices = INDICES.len() as u32;

        log::info!("MprRenderContext initialized with shared GPU resources");

        Self {
            render_pipeline,
            texture_bind_group_layout,
            vertex_bind_group_layout,
            fragment_bind_group_layout,
            vertex_buffer,
            index_buffer,
            num_indices,
        }
    }
}