#![allow(dead_code)]

use wgpu::{Device, Queue};

pub struct MeshRenderContext {
    pub pipeline: std::sync::Arc<wgpu::RenderPipeline>,
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub num_vertices: u32,
    pub num_indices: u32,
}

impl MeshRenderContext {
    /// Create a MeshRenderContext using the centralized mesh pipeline helper with indexed triangle rendering.
    /// This acquires a cached MeshBasic pipeline (TriangleList, CCW front face) with depth testing enabled.
    /// It uploads both vertex and index buffers for efficient triangle rasterization across native and WASM.
    pub fn new(manager: &mut crate::pipeline::PipelineManager, device: &Device, queue: &Queue, mesh: &super::mesh::Mesh) -> Self {
        // Acquire cached/basic mesh pipeline via centralized helper (TriangleList topology and depth enabled)
        let pipeline = crate::pipeline::get_or_create_mesh_pipeline(manager, device);

        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Mesh Vertex Buffer"),
            size: (mesh.vertices.len() * std::mem::size_of::<super::mesh::MeshVertex>()) as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        queue.write_buffer(&vertex_buffer, 0, bytemuck::cast_slice(&mesh.vertices));
        let num_vertices = mesh.vertices.len() as u32;

        let index_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Mesh Index Buffer"),
            size: (mesh.indices.len() * std::mem::size_of::<u32>()) as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        if !mesh.indices.is_empty() {
            queue.write_buffer(&index_buffer, 0, bytemuck::cast_slice(&mesh.indices));
        }
        let num_indices = mesh.indices.len() as u32;

        Self { pipeline, vertex_buffer, index_buffer, num_vertices, num_indices }
    }
}