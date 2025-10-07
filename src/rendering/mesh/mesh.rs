#![allow(dead_code)]

use bytemuck::{Pod, Zeroable};

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct MeshVertex {
    pub position: [f32; 3],
    pub color: [f32; 3],
}

// Manual implementation to avoid potential bytemuck derive issues
unsafe impl bytemuck::Zeroable for MeshVertex {}
unsafe impl bytemuck::Pod for MeshVertex {}

#[derive(Default, Debug, Clone)]
pub struct Mesh {
    pub vertices: Vec<MeshVertex>,
    pub indices: Vec<u16>,
}

impl Mesh {
    /// Function-level comment: Creates a simple unit cube mesh matching temp/main.rs structure
    /// Returns a cube with 8 vertices and 12 triangles for basic 3D rendering
    pub fn unit_cube() -> Self {
        // Create cube vertices matching temp/main.rs structure
        let vertices = vec![
            // front face
            MeshVertex { position: [-1.0, -1.0,  1.0], color: [1.0, 0.0, 0.0] }, // 0: front bottom-left (red)
            MeshVertex { position: [ 1.0, -1.0,  1.0], color: [0.0, 1.0, 0.0] }, // 1: front bottom-right (green)
            MeshVertex { position: [ 1.0,  1.0,  1.0], color: [0.0, 0.0, 1.0] }, // 2: front top-right (blue)
            MeshVertex { position: [-1.0,  1.0,  1.0], color: [1.0, 1.0, 0.0] }, // 3: front top-left (yellow)
            // back face
            MeshVertex { position: [-1.0, -1.0, -1.0], color: [1.0, 0.0, 1.0] }, // 4: back bottom-left (magenta)
            MeshVertex { position: [ 1.0, -1.0, -1.0], color: [0.0, 1.0, 1.0] }, // 5: back bottom-right (cyan)
            MeshVertex { position: [ 1.0,  1.0, -1.0], color: [1.0, 1.0, 1.0] }, // 6: back top-right (white)
            MeshVertex { position: [-1.0,  1.0, -1.0], color: [0.2, 0.6, 0.2] }, // 7: back top-left (dark green)
        ];

        // Create cube indices matching temp/main.rs structure (CCW winding)
        let indices: Vec<u16> = vec![
            0,1,2, 2,3,0, // front face
            4,5,6, 6,7,4, // back face
            4,5,1, 1,0,4, // bottom face
            7,6,2, 2,3,7, // top face
            4,0,3, 3,7,4, // left face
            5,1,2, 2,6,5, // right face
        ];

        Self { vertices, indices }
    }
}

impl MeshVertex {
    /// Function-level comment: Vertex attribute array matching temp/main.rs structure
    /// Defines position and color attributes for the vertex shader
    const ATTRS: [wgpu::VertexAttribute; 2] = wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3];

    /// Function-level comment: Creates vertex buffer layout descriptor matching temp/main.rs
    /// Returns layout for position and color vertex attributes
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<MeshVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRS,
        }
    }
}