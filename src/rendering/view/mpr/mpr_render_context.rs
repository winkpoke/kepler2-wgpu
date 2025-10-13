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
        manager: &mut crate::rendering::core::pipeline::PipelineManager,
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

        // Create render pipeline using centralized cache helper
        let bgls: [&wgpu::BindGroupLayout; 3] = [
            &texture_bind_group_layout,
            &vertex_bind_group_layout,
            &fragment_bind_group_layout,
        ];
        let render_pipeline = crate::rendering::core::pipeline::get_or_create_texture_quad_pipeline(
            manager,
            device,
            bgls,
            &[Vertex::desc()],
            target_format,
        );

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