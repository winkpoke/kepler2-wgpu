#![allow(dead_code)]


/// Function-level comment: Minimal lighting uniform structure for basic mesh lighting MVP.
/// Supports a single directional light with ambient lighting for simple 3D illumination.
/// Uses proper 16-byte alignment for WGSL uniform buffers.
#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct BasicLightingUniforms {
    pub light_direction: [f32; 3],
    pub _padding1: f32,
    pub light_color: [f32; 3], 
    pub light_intensity: f32,
    pub ambient_color: [f32; 3],
    pub ambient_intensity: f32,
}

impl Default for BasicLightingUniforms {
    /// Function-level comment: Creates default lighting configuration for basic mesh rendering.
    /// Provides reasonable defaults for directional lighting from top-left-front direction.
    fn default() -> Self {
        Self {
            light_direction: [-0.5, -1.0, -0.5],  // Top-left-front direction
            _padding1: 0.0,
            light_color: [1.0, 1.0, 1.0],         // White light
            light_intensity: 1.0,
            ambient_color: [0.2, 0.2, 0.2],       // Dim ambient light
            ambient_intensity: 0.3,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct MeshVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
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

        // Define face normals for unit cube
        let front_normal = [0.0, 0.0, 1.0];   // +Z
        let back_normal = [0.0, 0.0, -1.0];   // -Z
        let bottom_normal = [0.0, -1.0, 0.0]; // -Y
        let top_normal = [0.0, 1.0, 0.0];     // +Y
        let left_normal = [-1.0, 0.0, 0.0];   // -X
        let right_normal = [1.0, 0.0, 0.0];   // +X

        let vertices = vec![
            // Front face (Red) - vertices 0-3
            MeshVertex { position: [-1.0, -1.0,  1.0], normal: front_normal, color: front_color }, // 0
            MeshVertex { position: [ 1.0, -1.0,  1.0], normal: front_normal, color: front_color }, // 1
            MeshVertex { position: [ 1.0,  1.0,  1.0], normal: front_normal, color: front_color }, // 2
            MeshVertex { position: [-1.0,  1.0,  1.0], normal: front_normal, color: front_color }, // 3
            
            // Back face (Green) - vertices 4-7
            MeshVertex { position: [ 1.0, -1.0, -1.0], normal: back_normal, color: back_color }, // 4
            MeshVertex { position: [-1.0, -1.0, -1.0], normal: back_normal, color: back_color }, // 5
            MeshVertex { position: [-1.0,  1.0, -1.0], normal: back_normal, color: back_color }, // 6
            MeshVertex { position: [ 1.0,  1.0, -1.0], normal: back_normal, color: back_color }, // 7
            
            // Bottom face (Blue) - vertices 8-11
            MeshVertex { position: [-1.0, -1.0, -1.0], normal: bottom_normal, color: bottom_color }, // 8
            MeshVertex { position: [ 1.0, -1.0, -1.0], normal: bottom_normal, color: bottom_color }, // 9
            MeshVertex { position: [ 1.0, -1.0,  1.0], normal: bottom_normal, color: bottom_color }, // 10
            MeshVertex { position: [-1.0, -1.0,  1.0], normal: bottom_normal, color: bottom_color }, // 11
            
            // Top face (Yellow) - vertices 12-15
            MeshVertex { position: [-1.0,  1.0,  1.0], normal: top_normal, color: top_color }, // 12
            MeshVertex { position: [ 1.0,  1.0,  1.0], normal: top_normal, color: top_color }, // 13
            MeshVertex { position: [ 1.0,  1.0, -1.0], normal: top_normal, color: top_color }, // 14
            MeshVertex { position: [-1.0,  1.0, -1.0], normal: top_normal, color: top_color }, // 15
            
            // Left face (Magenta) - vertices 16-19
            MeshVertex { position: [-1.0, -1.0, -1.0], normal: left_normal, color: left_color }, // 16
            MeshVertex { position: [-1.0, -1.0,  1.0], normal: left_normal, color: left_color }, // 17
            MeshVertex { position: [-1.0,  1.0,  1.0], normal: left_normal, color: left_color }, // 18
            MeshVertex { position: [-1.0,  1.0, -1.0], normal: left_normal, color: left_color }, // 19
            
            // Right face (Cyan) - vertices 20-23
            MeshVertex { position: [ 1.0, -1.0,  1.0], normal: right_normal, color: right_color }, // 20
            MeshVertex { position: [ 1.0, -1.0, -1.0], normal: right_normal, color: right_color }, // 21
            MeshVertex { position: [ 1.0,  1.0, -1.0], normal: right_normal, color: right_color }, // 22
            MeshVertex { position: [ 1.0,  1.0,  1.0], normal: right_normal, color: right_color }, // 23
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

    /// Function-level comment: Creates a unit cube with uniform colors for better lighting visualization
    /// Each face has the same uniform color that will be modified by lighting calculations in the shader
    pub fn uniform_color_cube() -> Self {
        // Use the same neutral gray color for all faces to better isolate lighting effects
        let uniform_color = [0.7, 0.2, 0.3];   

        // Define face normals for unit cube
        let front_normal = [0.0, 0.0, 1.0];   // +Z
        let back_normal = [0.0, 0.0, -1.0];   // -Z
        let bottom_normal = [0.0, -1.0, 0.0]; // -Y
        let top_normal = [0.0, 1.0, 0.0];     // +Y
        let left_normal = [-1.0, 0.0, 0.0];   // -X
        let right_normal = [1.0, 0.0, 0.0];   // +X

        let vertices = vec![
            // Front face (Uniform Gray) - vertices 0-3
            MeshVertex { position: [-1.0, -1.0,  1.0], normal: front_normal, color: uniform_color }, // 0
            MeshVertex { position: [ 1.0, -1.0,  1.0], normal: front_normal, color: uniform_color }, // 1
            MeshVertex { position: [ 1.0,  1.0,  1.0], normal: front_normal, color: uniform_color }, // 2
            MeshVertex { position: [-1.0,  1.0,  1.0], normal: front_normal, color: uniform_color }, // 3
            
            // Back face (Uniform Gray) - vertices 4-7
            MeshVertex { position: [ 1.0, -1.0, -1.0], normal: back_normal, color: uniform_color }, // 4
            MeshVertex { position: [-1.0, -1.0, -1.0], normal: back_normal, color: uniform_color }, // 5
            MeshVertex { position: [-1.0,  1.0, -1.0], normal: back_normal, color: uniform_color }, // 6
            MeshVertex { position: [ 1.0,  1.0, -1.0], normal: back_normal, color: uniform_color }, // 7
            
            // Bottom face (Uniform Gray) - vertices 8-11
            MeshVertex { position: [-1.0, -1.0, -1.0], normal: bottom_normal, color: uniform_color }, // 8
            MeshVertex { position: [ 1.0, -1.0, -1.0], normal: bottom_normal, color: uniform_color }, // 9
            MeshVertex { position: [ 1.0, -1.0,  1.0], normal: bottom_normal, color: uniform_color }, // 10
            MeshVertex { position: [-1.0, -1.0,  1.0], normal: bottom_normal, color: uniform_color }, // 11
            
            // Top face (Uniform Gray) - vertices 12-15
            MeshVertex { position: [-1.0,  1.0,  1.0], normal: top_normal, color: uniform_color }, // 12
            MeshVertex { position: [ 1.0,  1.0,  1.0], normal: top_normal, color: uniform_color }, // 13
            MeshVertex { position: [ 1.0,  1.0, -1.0], normal: top_normal, color: uniform_color }, // 14
            MeshVertex { position: [-1.0,  1.0, -1.0], normal: top_normal, color: uniform_color }, // 15
            
            // Left face (Uniform Gray) - vertices 16-19
            MeshVertex { position: [-1.0, -1.0, -1.0], normal: left_normal, color: uniform_color }, // 16
            MeshVertex { position: [-1.0, -1.0,  1.0], normal: left_normal, color: uniform_color }, // 17
            MeshVertex { position: [-1.0,  1.0,  1.0], normal: left_normal, color: uniform_color }, // 18
            MeshVertex { position: [-1.0,  1.0, -1.0], normal: left_normal, color: uniform_color }, // 19
            
            // Right face (Uniform Gray) - vertices 20-23
            MeshVertex { position: [ 1.0, -1.0,  1.0], normal: right_normal, color: uniform_color }, // 20
            MeshVertex { position: [ 1.0, -1.0, -1.0], normal: right_normal, color: uniform_color }, // 21
            MeshVertex { position: [ 1.0,  1.0, -1.0], normal: right_normal, color: uniform_color }, // 22
            MeshVertex { position: [ 1.0,  1.0,  1.0], normal: right_normal, color: uniform_color }, // 23
        ];

        // Create cube indices for each face (CCW winding) - all faces use uniform gray color
        let indices: Vec<u16> = vec![
            // Front face (Uniform Gray)
            0, 1, 2,  2, 3, 0,
            // Back face (Uniform Gray)
            4, 5, 6,  6, 7, 4,
            // Bottom face (Uniform Gray)
            8, 9, 10,  10, 11, 8,
            // Top face (Uniform Gray)
            12, 13, 14,  14, 15, 12,
            // Left face (Uniform Gray)
            16, 17, 18,  18, 19, 16,
            // Right face (Uniform Gray)
            20, 21, 22,  22, 23, 20,
        ];

        Self { vertices, indices }
    }
}

impl MeshVertex {
    /// Function-level comment: Vertex attribute array for position, normal, and color
    /// Defines position, normal, and color attributes for the vertex shader
    const ATTRS: [wgpu::VertexAttribute; 3] = wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3, 2 => Float32x3];

    /// Function-level comment: Creates vertex buffer layout descriptor for lighting-enabled mesh
    /// Returns layout for position, normal, and color vertex attributes
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<MeshVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRS,
        }
    }
}