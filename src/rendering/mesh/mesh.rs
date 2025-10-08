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
    /// Function-level comment: Creates a unit cube mesh with different colors for each face
    /// Returns a cube with 24 vertices (4 per face) and 12 triangles for colorful 3D rendering
    pub fn unit_cube() -> Self {
        // Define distinct colors for each face
        let front_color = [1.0, 0.0, 0.0];  // Red
        let back_color = [0.0, 1.0, 0.0];   // Green
        let bottom_color = [0.0, 0.0, 1.0]; // Blue
        let top_color = [1.0, 1.0, 0.0];    // Yellow
        let left_color = [1.0, 0.0, 1.0];   // Magenta
        let right_color = [0.0, 1.0, 1.0];  // Cyan

        let vertices = vec![
            // Front face (Red) - vertices 0-3
            MeshVertex { position: [-1.0, -1.0,  1.0], color: front_color }, // 0
            MeshVertex { position: [ 1.0, -1.0,  1.0], color: front_color }, // 1
            MeshVertex { position: [ 1.0,  1.0,  1.0], color: front_color }, // 2
            MeshVertex { position: [-1.0,  1.0,  1.0], color: front_color }, // 3
            
            // Back face (Green) - vertices 4-7
            MeshVertex { position: [ 1.0, -1.0, -1.0], color: back_color }, // 4
            MeshVertex { position: [-1.0, -1.0, -1.0], color: back_color }, // 5
            MeshVertex { position: [-1.0,  1.0, -1.0], color: back_color }, // 6
            MeshVertex { position: [ 1.0,  1.0, -1.0], color: back_color }, // 7
            
            // Bottom face (Blue) - vertices 8-11
            MeshVertex { position: [-1.0, -1.0, -1.0], color: bottom_color }, // 8
            MeshVertex { position: [ 1.0, -1.0, -1.0], color: bottom_color }, // 9
            MeshVertex { position: [ 1.0, -1.0,  1.0], color: bottom_color }, // 10
            MeshVertex { position: [-1.0, -1.0,  1.0], color: bottom_color }, // 11
            
            // Top face (Yellow) - vertices 12-15
            MeshVertex { position: [-1.0,  1.0,  1.0], color: top_color }, // 12
            MeshVertex { position: [ 1.0,  1.0,  1.0], color: top_color }, // 13
            MeshVertex { position: [ 1.0,  1.0, -1.0], color: top_color }, // 14
            MeshVertex { position: [-1.0,  1.0, -1.0], color: top_color }, // 15
            
            // Left face (Magenta) - vertices 16-19
            MeshVertex { position: [-1.0, -1.0, -1.0], color: left_color }, // 16
            MeshVertex { position: [-1.0, -1.0,  1.0], color: left_color }, // 17
            MeshVertex { position: [-1.0,  1.0,  1.0], color: left_color }, // 18
            MeshVertex { position: [-1.0,  1.0, -1.0], color: left_color }, // 19
            
            // Right face (Cyan) - vertices 20-23
            MeshVertex { position: [ 1.0, -1.0,  1.0], color: right_color }, // 20
            MeshVertex { position: [ 1.0, -1.0, -1.0], color: right_color }, // 21
            MeshVertex { position: [ 1.0,  1.0, -1.0], color: right_color }, // 22
            MeshVertex { position: [ 1.0,  1.0,  1.0], color: right_color }, // 23
        ];

        // Create cube indices for each colored face (CCW winding)
        let indices: Vec<u16> = vec![
            // Front face (Red)
            0, 1, 2,  2, 3, 0,
            // Back face (Green)
            4, 5, 6,  6, 7, 4,
            // Bottom face (Blue)
            8, 9, 10,  10, 11, 8,
            // Top face (Yellow)
            12, 13, 14,  14, 15, 12,
            // Left face (Magenta)
            16, 17, 18,  18, 19, 16,
            // Right face (Cyan)
            20, 21, 22,  22, 23, 20,
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