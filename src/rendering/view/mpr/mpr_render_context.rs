#![allow(dead_code)]

use std::sync::Arc;
use wgpu::util::DeviceExt;
use crate::rendering::pipeline::*;
use crate::rendering::view::render_content::RenderContent;

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

    /// Default "empty" segmentation texture
    pub default_seg_content: Arc<RenderContent>,
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
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue) -> Self {
        // Create bind group layout for 3D texture and sampler
        let texture_bind_group_layout = create_texture_bind_group_layout_addseg(device);

        // Create bind group layouts for uniforms
        let vertex_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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

        let fragment_bind_group_layout = create_uniform_bind_group_layout(device, None);

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

        // Build the 1x1x1 "empty" segmentation texture used as the default
        // for binding 2/3 until an AI result has been uploaded. A single
        // zero byte is uploaded so the texture is a valid R8Uint image.
        let default_seg_content = Arc::new(
            RenderContent::from_labels_r8(device, queue, &[0u8], "MPR Default Seg", 1, 1, 1)
            .expect("failed to build default seg content"),
        );

        log::info!("MprRenderContext initialized with shared GPU resources");

        Self {
            render_pipeline,
            texture_bind_group_layout,
            vertex_bind_group_layout,
            fragment_bind_group_layout,
            vertex_buffer,
            index_buffer,
            num_indices,
            default_seg_content,
        }
    }
}
