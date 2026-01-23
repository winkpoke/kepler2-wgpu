#![allow(dead_code)]

use super::mesh::BasicLightingUniforms;
use crate::rendering::core::pipeline::{
    create_basic_lighting_bind_group_layout, create_basic_mesh_pipeline_with_lighting,
};
use wgpu::util::DeviceExt;
use wgpu::{Device, Queue};

/// Function-level comment: Simplified uniform data structure for basic mesh rendering
/// Contains only a single combined model-view-projection matrix for efficient transformation
#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct BasicUniforms {
    pub model_view_proj: [[f32; 4]; 4],
}

impl Default for BasicUniforms {
    fn default() -> Self {
        Self {
            model_view_proj: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
        }
    }
}

/// Function-level comment: Basic mesh context for simplified rendering operations
/// This struct provides a minimal interface for mesh rendering without complex features
pub struct BasicMeshContext {
    pub pipeline: std::sync::Arc<wgpu::RenderPipeline>,
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub num_vertices: u32,
    pub num_indices: u32,
    // Simplified uniform handling - only basic uniforms needed
    pub uniform_buffer: wgpu::Buffer,
    pub bind_group: wgpu::BindGroup,
    /// Function-level comment: Uniform buffer for lighting parameters
    pub lighting_uniform_buffer: wgpu::Buffer,
    pub lighting_bind_group: wgpu::BindGroup,
}

impl BasicMeshContext {
    /// Create a new basic mesh context with simplified pipeline and uniforms
    pub fn new(device: &Device, queue: &Queue, mesh: &super::mesh::Mesh, use_depth: bool) -> Self {
        log::info!(
            "Creating BasicMeshContext with {} vertices, {} indices, depth: {}",
            mesh.vertices.len(),
            mesh.indices.len(),
            use_depth
        );

        // Create vertex buffer
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Basic Mesh Vertex Buffer"),
            contents: bytemuck::cast_slice(&mesh.vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        // Create index buffer
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Basic Mesh Index Buffer"),
            contents: bytemuck::cast_slice(&mesh.indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        // Create uniform buffer
        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Basic Mesh Uniform Buffer"),
            size: std::mem::size_of::<BasicUniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Initialize with default uniforms
        let default_uniforms = BasicUniforms::default();
        queue.write_buffer(
            &uniform_buffer,
            0,
            bytemuck::cast_slice(&[default_uniforms]),
        );

        // Create bind group layout for basic uniforms
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Basic Mesh Bind Group Layout"),
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

        // Create lighting uniform buffer
        let lighting_uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Basic Lighting Uniform Buffer"),
            size: std::mem::size_of::<BasicLightingUniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Initialize with default lighting uniforms
        let default_lighting_uniforms = BasicLightingUniforms::default();
        queue.write_buffer(
            &lighting_uniform_buffer,
            0,
            bytemuck::cast_slice(&[default_lighting_uniforms]),
        );

        // Create lighting bind group layout
        let lighting_bind_group_layout = create_basic_lighting_bind_group_layout(device);

        // Create lighting-enabled mesh pipeline with depth testing enabled to match render pass
        let pipeline = std::sync::Arc::new(create_basic_mesh_pipeline_with_lighting(
            device,
            &bind_group_layout,
            &lighting_bind_group_layout,
            use_depth,
        ));

        // Create bind group for uniforms
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Basic Mesh Bind Group"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        // Create bind group for lighting
        let lighting_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Basic Lighting Bind Group"),
            layout: &lighting_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: lighting_uniform_buffer.as_entire_binding(),
            }],
        });

        log::info!("BasicMeshContext created successfully");

        Self {
            pipeline,
            vertex_buffer,
            index_buffer,
            num_vertices: mesh.vertices.len() as u32,
            num_indices: mesh.indices.len() as u32,
            uniform_buffer,
            bind_group,
            lighting_uniform_buffer,
            lighting_bind_group,
        }
    }

    /// Function-level comment: Update uniforms with combined model-view-projection matrix
    pub fn update_uniforms(&self, queue: &Queue, model_view_proj_matrix: &[[f32; 4]; 4]) {
        let uniforms = BasicUniforms {
            model_view_proj: *model_view_proj_matrix,
        };

        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[uniforms]));
        log::trace!("Updated basic mesh uniforms with MVP matrix");
    }

    pub fn update_lighting(&self, queue: &Queue, uniforms: BasicLightingUniforms) {
        queue.write_buffer(
            &self.lighting_uniform_buffer,
            0,
            bytemuck::cast_slice(&[uniforms]),
        );
    }

    /// Function-level comment: Render the mesh using the basic pipeline with lighting
    pub fn render(&self, render_pass: &mut wgpu::RenderPass) {
        // Single debug log per render call instead of 6 separate logs
        log::debug!(
            "[BASIC_MESH_RENDER] Rendering mesh: {} indices, {} vertices",
            self.num_indices,
            self.num_vertices
        );

        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &self.bind_group, &[]);
        render_pass.set_bind_group(1, &self.lighting_bind_group, &[]);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        render_pass.draw_indexed(0..self.num_indices, 0, 0..1);

        log::trace!("BasicMeshContext::render - Draw call completed");
    }

    /// Function-level comment: Get basic memory statistics for the mesh context
    /// Returns (vertex_buffer_size, index_buffer_size, 0.0, 0.0) for compatibility
    pub fn get_memory_stats(&self) -> (u64, u64, f32, f32) {
        let vertex_size = self.vertex_buffer.size();
        let index_size = self.index_buffer.size();
        (vertex_size, index_size, 0.0, 0.0)
    }
}
