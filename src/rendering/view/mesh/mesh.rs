#![allow(dead_code)]

use crate::data::CTVolume;
use super::mesh_processing::*;
use ndarray::Array3;


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
    pub padding2: [f32; 3],
    pub opacity: f32,
}

impl Default for BasicLightingUniforms {
    /// Function-level comment: Creates default lighting configuration for basic mesh rendering.
    /// Provides reasonable defaults for directional lighting from top-left-front direction.
    fn default() -> Self {
        Self {
            light_direction: [0.6, -0.7, 0.3],  // Top-left-front direction
            _padding1: 0.0,
            light_color: [1.0, 1.0, 1.0],         // White light
            light_intensity: 1.0,
            ambient_color: [0.4, 0.4, 0.4],
            ambient_intensity: 0.5,
            padding2: [0.0, 0.0, 0.0],
            opacity: 1.0,
        }
    }
}

/// Function-level comment: High-level lighting configuration for mesh rendering.
/// Provides a simple interface for setting light direction and intensity.
#[derive(Default, Debug, Clone)]
pub struct Lighting {
    pub direction: [f32; 3],
    pub intensity: f32,
}

impl Lighting {
    /// Function-level comment: Create a new lighting configuration with specified direction and intensity.
    pub fn new(direction: [f32; 3], intensity: f32) -> Self {
        Self { direction, intensity }
    }

    /// Function-level comment: Convert high-level Lighting to BasicLightingUniforms for GPU upload.
    /// Maps the simple direction/intensity to the full uniform structure.
    pub fn to_basic_uniforms(&self) -> BasicLightingUniforms {
        BasicLightingUniforms {
            light_direction: self.direction,
            _padding1: 0.0,
            light_color: [1.0, 1.0, 1.0],         // White light
            light_intensity: self.intensity,
            ambient_color: [0.4, 0.4, 0.5],
            ambient_intensity: 0.4,
            padding2: [0.0, 0.0, 0.0],
            opacity: 1.0,
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
    pub indices: Vec<u32>,
}

struct Tissue {
    name: String,
    threshold: i16,
    color: [f32; 3],
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
        let indices: Vec<u32> = vec![
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
        let indices: Vec<u32> = vec![
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

    /// Function-level comment: Creates a simplified spine vertebra mesh for medical imaging visualization
    /// Represents a thoracic vertebra with body, arch, and processes for anatomical reference
    pub fn spine_vertebra() -> Self {
        // Vertebra bone color (light beige/cream)
        let bone_color = [0.9, 0.85, 0.75];
        
        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        let mut vertex_index = 0u16;
        
        // Helper function to add a box with proper normals
        let mut add_box = |center: [f32; 3], size: [f32; 3], color: [f32; 3]| {
            let half_size = [size[0] * 0.5, size[1] * 0.5, size[2] * 0.5];
            
            // Define face normals
            let front_normal = [0.0, 0.0, 1.0];
            let back_normal = [0.0, 0.0, -1.0];
            let bottom_normal = [0.0, -1.0, 0.0];
            let top_normal = [0.0, 1.0, 0.0];
            let left_normal = [-1.0, 0.0, 0.0];
            let right_normal = [1.0, 0.0, 0.0];
            
            // Define vertices relative to center
            let base_index = vertex_index;
            
            // Front face vertices
            vertices.push(MeshVertex {
                position: [center[0] - half_size[0], center[1] - half_size[1], center[2] + half_size[2]],
                normal: front_normal,
                color,
            });
            vertices.push(MeshVertex {
                position: [center[0] + half_size[0], center[1] - half_size[1], center[2] + half_size[2]],
                normal: front_normal,
                color,
            });
            vertices.push(MeshVertex {
                position: [center[0] + half_size[0], center[1] + half_size[1], center[2] + half_size[2]],
                normal: front_normal,
                color,
            });
            vertices.push(MeshVertex {
                position: [center[0] - half_size[0], center[1] + half_size[1], center[2] + half_size[2]],
                normal: front_normal,
                color,
            });
            
            // Back face vertices
            vertices.push(MeshVertex {
                position: [center[0] + half_size[0], center[1] - half_size[1], center[2] - half_size[2]],
                normal: back_normal,
                color,
            });
            vertices.push(MeshVertex {
                position: [center[0] - half_size[0], center[1] - half_size[1], center[2] - half_size[2]],
                normal: back_normal,
                color,
            });
            vertices.push(MeshVertex {
                position: [center[0] - half_size[0], center[1] + half_size[1], center[2] - half_size[2]],
                normal: back_normal,
                color,
            });
            vertices.push(MeshVertex {
                position: [center[0] + half_size[0], center[1] + half_size[1], center[2] - half_size[2]],
                normal: back_normal,
                color,
            });
            
            // Bottom face vertices
            vertices.push(MeshVertex {
                position: [center[0] - half_size[0], center[1] - half_size[1], center[2] - half_size[2]],
                normal: bottom_normal,
                color,
            });
            vertices.push(MeshVertex {
                position: [center[0] + half_size[0], center[1] - half_size[1], center[2] - half_size[2]],
                normal: bottom_normal,
                color,
            });
            vertices.push(MeshVertex {
                position: [center[0] + half_size[0], center[1] - half_size[1], center[2] + half_size[2]],
                normal: bottom_normal,
                color,
            });
            vertices.push(MeshVertex {
                position: [center[0] - half_size[0], center[1] - half_size[1], center[2] + half_size[2]],
                normal: bottom_normal,
                color,
            });
            
            // Top face vertices
            vertices.push(MeshVertex {
                position: [center[0] - half_size[0], center[1] + half_size[1], center[2] + half_size[2]],
                normal: top_normal,
                color,
            });
            vertices.push(MeshVertex {
                position: [center[0] + half_size[0], center[1] + half_size[1], center[2] + half_size[2]],
                normal: top_normal,
                color,
            });
            vertices.push(MeshVertex {
                position: [center[0] + half_size[0], center[1] + half_size[1], center[2] - half_size[2]],
                normal: top_normal,
                color,
            });
            vertices.push(MeshVertex {
                position: [center[0] - half_size[0], center[1] + half_size[1], center[2] - half_size[2]],
                normal: top_normal,
                color,
            });
            
            // Left face vertices
            vertices.push(MeshVertex {
                position: [center[0] - half_size[0], center[1] - half_size[1], center[2] - half_size[2]],
                normal: left_normal,
                color,
            });
            vertices.push(MeshVertex {
                position: [center[0] - half_size[0], center[1] - half_size[1], center[2] + half_size[2]],
                normal: left_normal,
                color,
            });
            vertices.push(MeshVertex {
                position: [center[0] - half_size[0], center[1] + half_size[1], center[2] + half_size[2]],
                normal: left_normal,
                color,
            });
            vertices.push(MeshVertex {
                position: [center[0] - half_size[0], center[1] + half_size[1], center[2] - half_size[2]],
                normal: left_normal,
                color,
            });
            
            // Right face vertices
            vertices.push(MeshVertex {
                position: [center[0] + half_size[0], center[1] - half_size[1], center[2] + half_size[2]],
                normal: right_normal,
                color,
            });
            vertices.push(MeshVertex {
                position: [center[0] + half_size[0], center[1] - half_size[1], center[2] - half_size[2]],
                normal: right_normal,
                color,
            });
            vertices.push(MeshVertex {
                position: [center[0] + half_size[0], center[1] + half_size[1], center[2] - half_size[2]],
                normal: right_normal,
                color,
            });
            vertices.push(MeshVertex {
                position: [center[0] + half_size[0], center[1] + half_size[1], center[2] + half_size[2]],
                normal: right_normal,
                color,
            });
            
            // Add indices for each face
            indices.extend_from_slice(&[
                // Front face
                (base_index + 0) as u32, (base_index + 1) as u32, (base_index + 2) as u32,  (base_index + 2) as u32, (base_index + 3) as u32, (base_index + 0) as u32,
                // Back face
                (base_index + 4) as u32, (base_index + 5) as u32, (base_index + 6) as u32,  (base_index + 6) as u32, (base_index + 7) as u32, (base_index + 4) as u32,
                // Bottom face
                (base_index + 8) as u32, (base_index + 9) as u32, (base_index + 10) as u32,  (base_index + 10) as u32, (base_index + 11) as u32, (base_index + 8) as u32,
                // Top face
                (base_index + 12) as u32, (base_index + 13) as u32, (base_index + 14) as u32,  (base_index + 14) as u32, (base_index + 15) as u32, (base_index + 12) as u32,
                // Left face
                (base_index + 16) as u32, (base_index + 17) as u32, (base_index + 18) as u32,  (base_index + 18) as u32, (base_index + 19) as u32, (base_index + 16) as u32,
                // Right face
                (base_index + 20) as u32, (base_index + 21) as u32, (base_index + 22) as u32,  (base_index + 22) as u32, (base_index + 23) as u32, (base_index + 20) as u32,
            ]);
            
            vertex_index += 24;
        };
        
        // Vertebra body (main cylindrical part)
        add_box([0.0, 0.0, 0.0], [1.2, 0.8, 1.0], bone_color);
        
        // Vertebral arch (posterior ring structure)
        add_box([0.0, 0.0, -0.8], [1.0, 0.6, 0.4], bone_color);
        
        // Spinous process (posterior projection)
        add_box([0.0, 0.0, -1.4], [0.4, 1.2, 0.3], bone_color);
        
        // Transverse processes (lateral projections)
        add_box([-0.9, 0.0, -0.4], [0.3, 0.4, 0.8], bone_color);
        add_box([0.9, 0.0, -0.4], [0.3, 0.4, 0.8], bone_color);
        
        // Superior articular processes
        add_box([-0.5, 0.5, 0.2], [0.2, 0.3, 0.4], bone_color);
        add_box([0.5, 0.5, 0.2], [0.2, 0.3, 0.4], bone_color);
        
        // Inferior articular processes
        add_box([-0.5, -0.5, -0.8], [0.2, 0.3, 0.4], bone_color);
        add_box([0.5, -0.5, -0.8], [0.2, 0.3, 0.4], bone_color);
        
        Self { vertices, indices }
    }

    pub fn new(
        ctvolume: &CTVolume,
        iso_value: f32,
        window: Option<[f32; 2]>,
    ) -> Self {
        let smooth_iterations = 5;
        let smooth_lambda = 2.0;
        let downsample_grid_size = 8.0; // 2.0 mm grid cell size
        let tissues = vec![
            // Bone (Cortical) - Hard bone
            Tissue {
                name: "Bone_Cortical".to_string(),
                threshold: iso_value as i16,
                color: [0.95, 0.90, 0.85], // #F2E6D9
                // color: [0.16, 0.87, 0.60], // rgba(40, 221, 152, 1)
            },
            // Bone (Trabecular) - Spongy bone
            Tissue {
                name: "Bone_Trabecular".to_string(),
                threshold: 300,
                color: [0.85, 0.80, 0.75], // #D9CCBF
                // color: [0.16, 0.87, 0.47], // rgba(40, 221, 121, 1)
            },
            // Soft Tissue (Muscle)
            // Tissue {
            //     name: "Muscle".to_string(),
            //     threshold: 40,
            //     color: [0.80, 0.40, 0.40], // #CC6666
            // },
        ];
        println!("Starting Marching Tetrahedra conversion...");

        // 1. Read DICOM
        let dimensions = ctvolume.dimensions;
        let spacing = (
            ctvolume.voxel_spacing.2 as f64, // spacing_z
            ctvolume.voxel_spacing.1 as f64, // spacing_y
            ctvolume.voxel_spacing.0 as f64, // spacing_x
        );
        // Apply WL/WW windowing to voxel data if provided, to make WW 和 WL 对几何的影响彼此独立
        let windowed_data = if let Some([wl, ww]) = window {
            let lower = (wl - ww * 0.5).round() as i16;
            let upper = (wl + ww * 0.5).round() as i16;
            let mut v = Vec::with_capacity(ctvolume.voxel_data.len());
            for &val in &ctvolume.voxel_data {
                if val < lower || val > upper { v.push(lower - 1); } else { v.push(val); }
            }
            v
        } else {
            // Avoid cloning if possible, but here we need a local copy for processing
            // In a future optimization, we could pass the slice directly if no windowing is needed
            ctvolume.voxel_data.clone()
        };

        let volume = match Array3::from_shape_vec((dimensions.2, dimensions.1, dimensions.0), windowed_data) {
            Ok(arr) => arr,
            Err(e) => {
                log::warn!(
                    "volume shape mismatch: expected {} elements, got {}; error: {}",
                    dimensions.2 * dimensions.1 * dimensions.0,
                    ctvolume.voxel_data.len(),
                    e
                );
                return Self { vertices: Vec::new(), indices: Vec::new() };
            }
        };

        // 2. Extract Surfaces
        let mut meshes = Vec::new();
        // let mut meshes_out = Vec::new();
        for tissue in tissues {
            println!("\n=== Processing {} (Threshold={}) ===", tissue.name, tissue.threshold);
            let mt = MarchingTetrahedra::new(tissue.threshold);
            let (vertices, faces) = mt.extract_surface(&volume, spacing);
            
            println!("Generated {} vertices, {} triangles", vertices.len(), faces.len());

            if !vertices.is_empty() {
                // Merge vertices to fix connectivity
                let (merged_vertices, merged_faces) = merge_vertices(&vertices, &faces);

                // Apply smoothing (match OBJ generation path)
                let smoothed_vertices = laplacian_smooth(
                    &merged_vertices,
                    &merged_faces,
                    smooth_iterations as usize,
                    smooth_lambda as f64,
                );
                let (downsampled_vertices, downsampled_faces) = downsample_mesh(&smoothed_vertices, &merged_faces, downsample_grid_size);

                // smoothed_vertices are already in millimeters from Marching Tetrahedra
                let mm_vertices: Vec<[f64; 3]> = downsampled_vertices.clone();

                let mut min_mm = [f64::INFINITY; 3];
                let mut max_mm = [f64::NEG_INFINITY; 3];
                for p in &mm_vertices {
                    for i in 0..3 {
                        min_mm[i] = min_mm[i].min(p[i]);
                        max_mm[i] = max_mm[i].max(p[i]);
                    }
                }
                let center_mm = [
                    (min_mm[0] + max_mm[0]) * 0.5,
                    (min_mm[1] + max_mm[1]) * 0.5,
                    (min_mm[2] + max_mm[2]) * 0.5,
                ];
                let extent_mm = [
                    max_mm[0] - min_mm[0],
                    max_mm[1] - min_mm[1],
                    max_mm[2] - min_mm[2],
                ];
                let max_extent_mm = extent_mm.iter().fold(0.0f64, |a, &b| a.max(b));
                if max_extent_mm < f64::EPSILON {
                    log::warn!("extract_isosurface: empty bounding box (mm), skipping.");
                }

                let scale = 2.0 / max_extent_mm;

                let normals = compute_vertex_normals_local(&mm_vertices, &downsampled_faces);

                let mut mesh_vertices = Vec::with_capacity(mm_vertices.len());
                for (idx, p) in mm_vertices.iter().enumerate() {
                    let x = (p[0] - center_mm[0]) * scale;
                    let y = (p[1] - center_mm[1]) * scale;
                    let z = (p[2] - center_mm[2]) * scale;
                    let normal = normals.get(idx).copied().unwrap_or([0.0, 0.0, 1.0]);
                    let color = tissue.color;
                    mesh_vertices.push(MeshVertex {
                        position: [x as f32, y as f32, z as f32],
                        normal,
                        color,
                    });
                }

                // generate final GPU indices (with consistent vertex order)
                let mesh_triangles: Vec<u32> = downsampled_faces
                    .into_iter()
                    .flat_map(|tri| tri.into_iter().map(|i| i as u32))
                    .collect();

                log::info!(
                    "extract_isosurface: {} vertices, {} triangles (normalized to unit cube)",
                    mesh_vertices.len(),
                    mesh_triangles.len() / 3
                );

                // Calculate mesh bounds for debugging
                let mut min_bounds = [f32::INFINITY, f32::INFINITY, f32::INFINITY];
                let mut max_bounds = [f32::NEG_INFINITY, f32::NEG_INFINITY, f32::NEG_INFINITY];
                
                for vertex in &mesh_vertices {
                    for i in 0..3 {
                        min_bounds[i] = min_bounds[i].min(vertex.position[i]);
                        max_bounds[i] = max_bounds[i].max(vertex.position[i]);
                    }
                }

                log::info!(
                    "extract_isosurface: {} vertices, {} triangles (normalized to unit cube)",
                    mesh_vertices.len(),
                    mesh_triangles.len() / 3
                );

                // // Convert MeshVertex -> positions as f64 for OBJ-style export
                // let vertices_pos: Vec<[f64; 3]> = mesh_vertices
                //     .iter()
                //     .map(|v| [v.position[0] as f64, v.position[1] as f64, v.position[2] as f64])
                //     .collect();

                // // Convert flat u32 indices -> face triplets as usize
                // let faces_usize: Vec<[usize; 3]> = mesh_triangles
                //     .chunks(3)
                //     .map(|c| [c[0] as usize, c[1] as usize, c[2] as usize])
                //     .collect();

                meshes.push(Self { vertices: mesh_vertices, indices: mesh_triangles });

                // meshes_out.push(NamedMesh {
                //     name: tissue.name,
                //     color: tissue.color,
                //     vertices: vertices_pos,
                //     faces: faces_usize,
                // });

                // let output_file = "output_mesh_mt.obj";
                // save_obj(output_file, &meshes_out).unwrap();
            } else {
                println!("Warning: No mesh generated for {}", tissue.name);
            }
        }
        
        if meshes.is_empty() {
            log::warn!("No mesh generated for any tissue.");
            return Self { vertices: Vec::new(), indices: Vec::new() };
        }

        let mut merged_vertices: Vec<MeshVertex> = Vec::new();
        let mut merged_indices: Vec<u32> = Vec::new();
        let mut base: u32 = 0;
        for m in meshes {
            let n = m.vertices.len() as u32;
            merged_vertices.extend(m.vertices.into_iter());
            merged_indices.extend(m.indices.into_iter().map(|i| i + base));
            base += n;
        }
        Self { vertices: merged_vertices, indices: merged_indices }
    }
}

/// Function-level comment: Accumulate face normals and normalize to per-vertex normals
fn compute_vertex_normals_local(vertices: &[[f64; 3]], faces: &[[usize; 3]]) -> Vec<[f32; 3]> {
    let mut acc: Vec<[f64; 3]> = vec![[0.0, 0.0, 0.0]; vertices.len()];
    for face in faces {
        let i0 = face[0];
        let i1 = face[1];
        let i2 = face[2];
        let p0 = vertices[i0];
        let p1 = vertices[i1];
        let p2 = vertices[i2];
        let v1 = [p1[0] - p0[0], p1[1] - p0[1], p1[2] - p0[2]];
        let v2 = [p2[0] - p0[0], p2[1] - p0[1], p2[2] - p0[2]];
        let n = [
            v1[1] * v2[2] - v1[2] * v2[1],
            v1[2] * v2[0] - v1[0] * v2[2],
            v1[0] * v2[1] - v1[1] * v2[0],
        ];
        acc[i0][0] += n[0]; acc[i0][1] += n[1]; acc[i0][2] += n[2];
        acc[i1][0] += n[0]; acc[i1][1] += n[1]; acc[i1][2] += n[2];
        acc[i2][0] += n[0]; acc[i2][1] += n[1]; acc[i2][2] += n[2];
    }
    acc.into_iter().map(|a| {
        let len = (a[0]*a[0] + a[1]*a[1] + a[2]*a[2]).sqrt();
        if len > 1e-12 { [ (a[0]/len) as f32, (a[1]/len) as f32, (a[2]/len) as f32 ] } else { [0.0, 0.0, 1.0] }
    }).collect()
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
