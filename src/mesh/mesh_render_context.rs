#![allow(dead_code)]

use wgpu::{Device, Queue};

pub struct MeshRenderContext {
    pub pipeline: std::sync::Arc<wgpu::RenderPipeline>,
    pub vertex_buffer: wgpu::Buffer,
    pub num_vertices: u32,
}

impl MeshRenderContext {
    /// Create a MeshRenderContext, using the provided PipelineManager for pipeline caching/creation.
    pub fn new(manager: &mut crate::pipeline::PipelineManager, device: &Device, queue: &Queue, mesh: &super::mesh::Mesh) -> Self {
        // Obtain cached/basic mesh pipeline via PipelineBuilder for centralized control
        #[allow(unused_imports)]
        use crate::pipeline_builder::{PipelineBuilder, PipelineRequest, PipelineParams, PipelineType};
        let mut builder = PipelineBuilder::new(device, manager);
        let params = PipelineParams { topology: Some(wgpu::PrimitiveTopology::PointList), ..Default::default() };
        let req = PipelineRequest { ty: PipelineType::MeshBasic, params };
        let pipeline = builder.build(&req).expect("Failed to build MeshBasic pipeline");

        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Mesh Vertex Buffer"),
            size: (mesh.vertices.len() * std::mem::size_of::<super::mesh::MeshVertex>()) as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        queue.write_buffer(&vertex_buffer, 0, bytemuck::cast_slice(&mesh.vertices));
        let num_vertices = mesh.vertices.len() as u32;

        Self { pipeline, vertex_buffer, num_vertices }
    }
}