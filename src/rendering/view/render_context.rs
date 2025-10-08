#![allow(dead_code)]

use crate::core::coord::{array_to_slice, Matrix4x4};
use crate::rendering::content::render_content::RenderContent;
use crate::rendering::{
    create_vertex_uniform_bind_group, get_or_create_texture_quad_pipeline, get_swapchain_format,
};
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Copy, Clone, Default, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct UniformsVert {
    // pub rotation_angle_y: f32,
    // pub rotation_angle_z: f32,
    pub _padding: [f32; 4],
}

#[repr(C)]
#[derive(Copy, Clone, Default, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct UniformsFrag {
    pub window_width: f32,
    pub window_level: f32,
    pub slice: f32,
    pub is_packed_rg8: f32,
    pub mat: [f32; 16],
}

#[repr(C)]
#[derive(Copy, Clone, Default, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Uniforms {
    pub(crate) vert: UniformsVert,
    pub(crate) frag: UniformsFrag,
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
pub struct RenderContext {
    pub render_pipeline: std::sync::Arc<wgpu::RenderPipeline>,
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub num_indices: u32,
    pub texture_bind_group: wgpu::BindGroup,
    pub uniform_vert_buffer: wgpu::Buffer,
    pub uniform_vert_bind_group: wgpu::BindGroup,
    pub uniform_frag_buffer: wgpu::Buffer,
    pub uniform_frag_bind_group: wgpu::BindGroup,
    pub uniforms: Uniforms,
}

impl RenderContext {
    /// Create a RenderContext using the centralized pipeline helper and PipelineManager cache.
    /// This unifies pipeline acquisition to a single path (no direct PipelineBuilder usage),
    /// ensuring consistent behavior across native and WASM targets.
    /// Parameters:
    /// - manager: PipelineManager cache used to retrieve/create the texture-quad pipeline.
    /// - device: wgpu device used for GPU resource creation.
    /// - texture: RenderContent whose 3D texture and sampler are bound to the fragment stage.
    /// - transform_matrix: 4x4 matrix applied to the vertex positions for view transforms.
    /// Returns: A fully initialized RenderContext with pipeline, buffers, bind groups, and uniforms.
    pub fn new(
        manager: &mut crate::rendering::core::pipeline::PipelineManager,
        device: &wgpu::Device,
        texture: &RenderContent,
        transform_matrix: Matrix4x4<f32>,
    ) -> RenderContext {
        let u_vert_data = UniformsVert {
            // rotation_angle_y: 0.0,
            // rotation_angle_z: 0.0,
            ..Default::default()
        };
        let is_packed = matches!(texture.texture_format, wgpu::TextureFormat::Rg8Unorm);
        let u_frag_data = UniformsFrag {
            window_width: 350.,
            window_level: if is_packed { 1140.0 } else { 40.0 },
            slice: 0.0,
            is_packed_rg8: if is_packed { 1.0 } else { 0.0 },
            mat: *array_to_slice(&transform_matrix.data),
            ..Default::default()
        };
        log::info!(
            "RenderContext defaults => window_width: {:.1}, window_level: {:.1}, is_packed_rg8: {}",
            u_frag_data.window_width,
            u_frag_data.window_level,
            is_packed
        );
        let uniforms = Uniforms {
            vert: u_vert_data,
            frag: u_frag_data,
        };
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
                        // This should match the filterable field of the
                        // corresponding Texture entry above.
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
                label: Some("texture_bind_group_layout"),
            });
        let texture_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &texture_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&texture.view), // CHANGED!
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&texture.sampler), // CHANGED!
                },
            ],
            label: Some("diffuse_bind_group"),
        });

        let (uniform_vert_buffer, uniform_vert_bind_group, vert_bind_group_layout) =
            crate::rendering::core::pipeline::create_vertex_uniform_bind_group(device, &uniforms.vert);

        let (uniform_frag_buffer, uniform_frag_bind_group, frag_bind_group_layout) =
            crate::rendering::core::pipeline::create_fragment_uniform_bind_group(
                device,
                &uniforms.frag,
            );

        let target_format = crate::rendering::core::pipeline::get_swapchain_format().unwrap_or(wgpu::TextureFormat::Rgba8Unorm);
        // Acquire pipeline via centralized cache helper to unify behavior with RenderApp
        let bgls: [&wgpu::BindGroupLayout; 3] = [
            &texture_bind_group_layout,
            &vert_bind_group_layout,
            &frag_bind_group_layout,
        ];
        let render_pipeline = crate::rendering::core::pipeline::get_or_create_texture_quad_pipeline(
            manager,
            device,
            bgls,
            &[Vertex::desc()],
            target_format,
        );

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(VERTICES),
            usage: wgpu::BufferUsages::VERTEX,
        });

        // let num_vertices = VERTICES.len() as u32;

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice(INDICES),
            usage: wgpu::BufferUsages::INDEX,
        });
        let num_indices = INDICES.len() as u32;

        Self {
            render_pipeline,
            vertex_buffer,
            index_buffer,
            num_indices,
            texture_bind_group,
            uniform_vert_buffer,
            uniform_vert_bind_group,
            uniform_frag_buffer,
            uniform_frag_bind_group,
            uniforms,
        }
    }
}
