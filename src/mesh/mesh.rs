#![allow(dead_code)]

use bytemuck::{Pod, Zeroable};

#[repr(C)]
#[derive(Clone, Copy, Debug, Zeroable, Pod)]
pub struct MeshVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub uv: [f32; 2],
}

#[derive(Default, Debug, Clone)]
pub struct Mesh {
    pub vertices: Vec<MeshVertex>,
    pub indices: Vec<u32>,
}

impl Mesh {
    pub fn unit_cube() -> Self {
        // Placeholder-only geometry; not optimized.
        // Simple 8-vertex cube with no indices for now.
        let mut vertices = Vec::new();
        let positions = [
            [-0.5, -0.5, -0.5],
            [ 0.5, -0.5, -0.5],
            [ 0.5,  0.5, -0.5],
            [-0.5,  0.5, -0.5],
            [-0.5, -0.5,  0.5],
            [ 0.5, -0.5,  0.5],
            [ 0.5,  0.5,  0.5],
            [-0.5,  0.5,  0.5],
        ];
        for p in positions.iter() {
            vertices.push(MeshVertex { position: *p, normal: [0.0, 0.0, 1.0], uv: [0.0, 0.0] });
        }
        Self { vertices, indices: Vec::new() }
    }
}

impl MeshVertex {
    pub const ATTRIBS: [wgpu::VertexAttribute; 3] =
        wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3, 2 => Float32x2];

    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<MeshVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBS,
        }
    }
}