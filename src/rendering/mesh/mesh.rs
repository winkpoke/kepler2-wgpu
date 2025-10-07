#![allow(dead_code)]

use bytemuck::{Pod, Zeroable};

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct MeshVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub uv: [f32; 2],
}

// Manual implementation to avoid potential bytemuck derive issues
unsafe impl bytemuck::Zeroable for MeshVertex {}
unsafe impl bytemuck::Pod for MeshVertex {}

#[derive(Default, Debug, Clone)]
pub struct Mesh {
    pub vertices: Vec<MeshVertex>,
    pub indices: Vec<u32>,
}

impl Mesh {
    /// Constructs a unit cube mesh composed of 12 triangles (CCW winding) suitable for TriangleList rasterization.
    /// Vertices are unit cube corners in clip-space range [-0.5, 0.5]; normals/uvs are placeholders for now.
    pub fn unit_cube() -> Self {
        // Placeholder-only geometry; not optimized.
        // Simple 8-vertex cube with indices for triangle faces.
        let mut vertices = Vec::new();
        let positions = [
            [-1.0, -1.0, -1.0], // 0
            [ 1.0, -1.0, -1.0], // 1
            [ 1.0,  1.0, -1.0], // 2
            [-1.0,  1.0, -1.0], // 3
            [-1.0, -1.0,  1.0], // 4
            [ 1.0, -1.0,  1.0], // 5
            [ 1.0,  1.0,  1.0], // 6
            [-1.0,  1.0,  1.0], // 7
        ];
        for p in positions.iter() {
            vertices.push(MeshVertex { position: *p, normal: [0.0, 0.0, 1.0], uv: [0.0, 0.0] });
        }
        // CCW triangles per face: back (-Z), front (+Z), left (-X), right (+X), bottom (-Y), top (+Y)
        let indices = vec![
            0, 1, 2, 0, 2, 3, // back
            4, 5, 6, 4, 6, 7, // front
            4, 0, 3, 4, 3, 7, // left
            1, 5, 6, 1, 6, 2, // right
            4, 5, 1, 4, 1, 0, // bottom
            3, 2, 6, 3, 6, 7, // top
        ];
        Self { vertices, indices }
    }
}

impl MeshVertex {
    // Temporarily comment out the macro to test if it's causing stack overflow
    // pub const ATTRIBS: [wgpu::VertexAttribute; 3] =
    //     wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3, 2 => Float32x2];

    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        // Manual vertex attributes to avoid potential macro issues
        const ATTRIBS: [wgpu::VertexAttribute; 3] = [
            wgpu::VertexAttribute {
                offset: 0,
                shader_location: 0,
                format: wgpu::VertexFormat::Float32x3,
            },
            wgpu::VertexAttribute {
                offset: 12,
                shader_location: 1,
                format: wgpu::VertexFormat::Float32x3,
            },
            wgpu::VertexAttribute {
                offset: 24,
                shader_location: 2,
                format: wgpu::VertexFormat::Float32x2,
            },
        ];
        
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<MeshVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &ATTRIBS,
        }
    }
}