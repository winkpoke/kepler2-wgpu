#![allow(dead_code)]

#[cfg(not(target_arch = "wasm32"))]
use rayon::prelude::*;
use crate::data::CTVolume;
use super::mesh_processing::*;
use ndarray::{Array3, s, ArrayView3};


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
#[derive(Debug, Clone)]
pub struct Lighting {
    pub direction: [f32; 3],
    pub light_color: [f32; 3],
    pub light_intensity: f32,
    pub ambient_color: [f32; 3],
    pub ambient_intensity: f32,
}

impl Default for Lighting {
    fn default() -> Self {
        Self {
            direction: [0.6, -0.7, 0.3],
            light_color: [1.0, 1.0, 1.0],
            light_intensity: 1.0,
            ambient_color: [0.4, 0.4, 0.5],
            ambient_intensity: 0.4,
        }
    }
}

impl Lighting {
    /// Function-level comment: Create a new lighting configuration with specified direction and intensity.
    pub fn new(direction: [f32; 3], intensity: f32) -> Self {
        Self { 
            direction, 
            light_intensity: intensity,
            ..Default::default()
        }
    }

    /// Function-level comment: Convert high-level Lighting to BasicLightingUniforms for GPU upload.
    /// Maps the simple direction/intensity to the full uniform structure.
    pub fn to_basic_uniforms(&self) -> BasicLightingUniforms {
        BasicLightingUniforms {
            light_direction: self.direction,
            _padding1: 0.0,
            light_color: self.light_color,
            light_intensity: self.light_intensity,
            ambient_color: self.ambient_color,
            ambient_intensity: self.ambient_intensity,
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
    min: i16,
    max: i16,
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
        iso_min: f32,
        iso_max: f32,
        world_min: Option<[f32; 3]>,
        world_max: Option<[f32; 3]>,
    ) -> Self {
        // Settings
        let smooth_iterations = 5;
        let smooth_lambda = 0.5;
        let downsample_grid_size = 2.0;

        // Gaussian Filter settings
        let enable_gaussian = true;
        let gaussian_sigma = 0.5;

        // Chunk settings
        let chunk_size = 500;
        let overlap = 4;

        // Tissue definitions (multi-color)
        let mut tissues = vec![ 
            Tissue {
                name: "Air".to_string(),
                min: -1000,
                max: -500,
                color: [0.6, 0.8, 1.0], // 淡蓝
            },
            Tissue {
                name: "Fat".to_string(),
                min: -200,
                max: -50,
                color: [0.9, 0.85, 0.75], // 浅棕/米色
            },
            Tissue {
                name: "Muscle".to_string(),
                min: 30,
                max: 80,
                color: [0.7, 0.55, 0.4], // 棕色（主肌肉色）
            },
            Tissue {
                name: "Organ".to_string(),
                min: 80,
                max: 150,
                color: [0.75, 0.6, 0.45], // 稍深的棕色（内脏，与肌肉有区分）
            },
            Tissue {
                name: "Bone_Cancellous".to_string(),
                min: 150,
                max: 300,
                color: [0.94, 0.90, 0.85], // 浅黄色（松质骨）
            },
            Tissue {
                name: "Bone_Cortical".to_string(),
                min: 300,
                max: 2000,
                color: [0.95, 0.90, 0.85], // 白（皮质骨）
            },
        ];


        // ✅ 根据 iso_min / iso_max 过滤组织
        if !tissues.is_empty() {
            tissues = tissues
                .into_iter()
                .filter(|t| {
                    // 只保留 HU 区间与 iso_min–iso_max 有重叠的组织
                    (t.max as f32) >= iso_min && (t.min as f32) <= iso_max
                })
                .collect();
            log::info!("Loaded {} tissues (filtered by iso_min={}, iso_max={})", tissues.len(), iso_min, iso_max);
        }else {
            tissues = vec![Tissue {
                name: "Generic".to_string(),
                min: iso_min as i16,
                max: iso_max as i16,
                color: [0.95, 0.90, 0.85],
            }];
        }

        // Read DICOM dims
        let (rows, columns, depth) = ctvolume.dimensions;
        let spacing = (
            ctvolume.voxel_spacing.2 as f64,
            ctvolume.voxel_spacing.1 as f64,
            ctvolume.voxel_spacing.0 as f64,
        );

        // ROI definition - Use arguments if provided, else default to full volume
        let roi_min = world_min.unwrap_or([f32::NEG_INFINITY; 3]);
        let roi_max = world_max.unwrap_or([f32::INFINITY; 3]);

        // ROI Z Range
        let (process_z_start, process_z_end) = if world_min.is_some() && world_max.is_some() {
            let origin_z = ctvolume.base.matrix.col(3)[2];
            let z_min_idx = ((roi_min[2] - origin_z) / ctvolume.voxel_spacing.2).floor() as isize;
            let z_max_idx = ((roi_max[2] - origin_z) / ctvolume.voxel_spacing.2).ceil() as isize;
            
            (z_min_idx.max(0) as usize, z_max_idx.min(depth as isize) as usize)
        } else {
            (0, depth)
        };
        
        // Downsampling logic
        let target_size = 256;
        let (new_rows, new_columns, scale_y, scale_x) = if rows > target_size || columns > target_size {
            let scale = (target_size as f64) / (rows.max(columns) as f64);
            let new_rows = (rows as f64 * scale).round() as usize;
            let new_columns = (columns as f64 * scale).round() as usize;
            (new_rows, new_columns, 1.0 / scale, 1.0 / scale)
        } else {
            (rows, columns, 1.0, 1.0)
        };

        let final_spacing_y = spacing.1 * scale_y;
        let final_spacing_x = spacing.2 * scale_x;

        // Prepare chunks
        let chunks: Vec<(usize, usize)> = (process_z_start..process_z_end)
            .step_by(chunk_size)
            .enumerate()
            .map(|(i, z)| (i, z))
            .collect();

        // Process chunk closure
        let process_chunk = |chunk_idx: usize, z_start: usize| -> (Vec<MeshVertex>, Vec<u32>) {
            let z_end = (z_start + chunk_size).min(process_z_end);
            if z_end <= z_start {
                return (Vec::new(), Vec::new());
            }

            log::info!("--- Processing Chunk {} (Slices {} to {}) ---", chunk_idx, z_start, z_end);

            // 1. Calculate Read Range (with Overlap)
            let read_start = z_start.saturating_sub(overlap);
            let read_end = (z_end + overlap).min(depth);
            
            if read_end - read_start < 2 {
                return (Vec::new(), Vec::new());
            }
            let chunk_depth = read_end - read_start;

            // 2. Build Chunk Volume (Parallelized Slice Loading)
            let mut chunk_volume = Array3::<i16>::zeros((chunk_depth, new_rows, new_columns));
            let vol = &ctvolume.voxel_data;

            for z in 0..chunk_depth {
                let global_z = read_start + z;
                let start = global_z * columns * rows;
                let end = start + columns * rows;
                let buf = &vol[start..end];
                let slice_vec = buf.to_vec();
                let slice_view = ndarray::Array2::from_shape_vec((rows, columns), slice_vec).unwrap_or_else(|_| ndarray::Array2::zeros((rows, columns)));

                let final_slice = if new_rows != rows || new_columns != columns {
                    resize_slice(&slice_view, (new_rows, new_columns))
                } else {
                    slice_view
                };

                chunk_volume.slice_mut(s![z, .., ..]).assign(&final_slice);
            }

            // 3. Gaussian Filter
            if enable_gaussian {
                chunk_volume = apply_gaussian_filter(chunk_volume, gaussian_sigma);
            }

            let (crop_x_start, crop_y_start, cropped_volume) = if let (Some(world_min), Some(world_max)) = (world_min, world_max) {
                // ROI logic
                let origin_x = ctvolume.base.matrix.col(3)[0];
                let origin_y = ctvolume.base.matrix.col(3)[1];

                let x_min_idx = ((roi_min[0] - origin_x) / final_spacing_x as f32).floor() as isize;
                let x_max_idx = ((roi_max[0] - origin_x) / final_spacing_x as f32).ceil() as isize;
                let x_dim = chunk_volume.len_of(ndarray::Axis(2)) as isize;
                let crop_x_start = x_min_idx.max(0) as usize;
                let crop_x_end = x_max_idx.min(x_dim) as usize;

                let y_min_idx = ((roi_min[1] - origin_y) / final_spacing_y as f32).floor() as isize;
                let y_max_idx = ((roi_max[1] - origin_y) / final_spacing_y as f32).ceil() as isize;
                let y_dim = chunk_volume.len_of(ndarray::Axis(1)) as isize;
                let crop_y_start = y_min_idx.max(0) as usize;
                let crop_y_end = y_max_idx.min(y_dim) as usize;
                
                if crop_x_end <= crop_x_start || crop_y_end <= crop_y_start {
                    log::warn!("Chunk {} outside ROI in X/Y plane, skipping.", chunk_idx);
                    return (Vec::new(), Vec::new());
                }

                let cropped_volume = chunk_volume.slice(s![.., crop_y_start..crop_y_end, crop_x_start..crop_x_end]);

                (crop_x_start, crop_y_start, cropped_volume)
            } else {
                (0, 0, chunk_volume.view())
            };

            // 4. Pinning
            let mut pinned_zs = Vec::new();
            let spacing_z = spacing.0 as f32;
            if z_start > 0 { pinned_zs.push(z_start as f32 * spacing_z); }
            if z_end < depth { pinned_zs.push(z_end as f32 * spacing_z); }

            // 5. Extract
            let local_start = z_start - read_start;
            let local_end = z_end - read_start;
            let extract_range = local_start..local_end;

            let mut local_vertices = Vec::new();
            let mut local_indices = Vec::new();

            for tissue in &tissues {
                let mt = MarchingTetrahedra::new(tissue.min, tissue.max);
                let offset = [crop_x_start, crop_y_start, read_start];
                let (vertices, faces) = mt.extract_surface(
                    &cropped_volume, 
                    (spacing.0, final_spacing_y, final_spacing_x),
                    offset,
                    extract_range.clone()
                );

                if !vertices.is_empty() {
                    let (merged_vertices, merged_faces) = merge_vertices(&vertices, &faces);
                    let smoothed_vertices = laplacian_smooth(&merged_vertices, &merged_faces, smooth_iterations, smooth_lambda, &pinned_zs);
                    let (final_chunk_vertices, final_chunk_faces) = downsample_mesh(&smoothed_vertices, &merged_faces, downsample_grid_size, &pinned_zs);
                    
                    let normals = compute_vertex_normals_local(&final_chunk_vertices, &final_chunk_faces);
                    
                    let base_index = local_vertices.len() as u32;
                    for (i, p) in final_chunk_vertices.iter().enumerate() {
                        local_vertices.push(MeshVertex {
                            position: *p,
                            normal: normals[i],
                            color: tissue.color,
                        });
                    }
                    
                    for tri in final_chunk_faces {
                        local_indices.push(tri[0] as u32 + base_index);
                        local_indices.push(tri[1] as u32 + base_index);
                        local_indices.push(tri[2] as u32 + base_index);
                    }
                }
            }
            (local_vertices, local_indices)
        };

        // Execute Chunks (Parallel or Serial)
        #[cfg(not(target_arch = "wasm32"))]
        let chunk_results: Vec<_> = chunks.par_iter().map(|&(i, z)| process_chunk(i, z)).collect();

        #[cfg(target_arch = "wasm32")]
        let chunk_results: Vec<_> = chunks.iter().map(|&(i, z)| process_chunk(i, z)).collect();

        // Merge Results
        let mut accumulated_vertices = Vec::new();
        let mut accumulated_indices = Vec::new();

        for (verts, indices) in chunk_results {
            let base = accumulated_vertices.len() as u32;
            accumulated_vertices.extend(verts);
            accumulated_indices.extend(indices.iter().map(|i| i + base));
        }

        if accumulated_vertices.is_empty() {
            log::warn!("No mesh generated for any tissue. Returning degenerate mesh to prevent buffer errors.");
            return Self { vertices: Vec::new(), indices: Vec::new() };
        }

        // Post-Process (Normalize)
        let mut min_mm = [f32::INFINITY; 3];
        let mut max_mm = [f32::NEG_INFINITY; 3];
        for v in &accumulated_vertices {
            for i in 0..3 {
                min_mm[i] = min_mm[i].min(v.position[i]);
                max_mm[i] = max_mm[i].max(v.position[i]);
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
        let max_extent_mm = extent_mm.iter().fold(0.0f32, |a, &b| a.max(b));
        
        let scale = if max_extent_mm > f32::EPSILON { 2.0 / max_extent_mm } else { 1.0 };
        
        for v in &mut accumulated_vertices {
             v.position[0] = (v.position[0] - center_mm[0]) * scale;
             v.position[1] = (v.position[1] - center_mm[1]) * scale;
             v.position[2] = (v.position[2] - center_mm[2]) * scale;
        }

        log::info!(
            "Generated Total: {} vertices, {} triangles (normalized)",
            accumulated_vertices.len(),
            accumulated_indices.len() / 3
        );

        Self { vertices: accumulated_vertices, indices: accumulated_indices }
    }
}

/// Resize a 2D slice to a new shape using bilinear interpolation.
fn resize_slice(data: &ndarray::Array2<i16>, new_shape: (usize, usize)) -> ndarray::Array2<i16> {
    let (old_rows, old_cols) = data.dim();
    let (new_rows, new_cols) = new_shape;
    let mut new_data = ndarray::Array2::<i16>::zeros((new_rows, new_cols));

    for r in 0..new_rows {
        for c in 0..new_cols {
            // Map new coordinates to old coordinates
            let old_r = r as f64 * (old_rows - 1) as f64 / (new_rows - 1).max(1) as f64;
            let old_c = c as f64 * (old_cols - 1) as f64 / (new_cols - 1).max(1) as f64;

            let r0 = old_r.floor() as usize;
            let c0 = old_c.floor() as usize;
            let r1 = (r0 + 1).min(old_rows - 1);
            let c1 = (c0 + 1).min(old_cols - 1);

            let dr = old_r - r0 as f64;
            let dc = old_c - c0 as f64;

            let v00 = data[[r0, c0]] as f64;
            let v01 = data[[r0, c1]] as f64;
            let v10 = data[[r1, c0]] as f64;
            let v11 = data[[r1, c1]] as f64;

            let v = v00 * (1.0 - dr) * (1.0 - dc)
                  + v01 * (1.0 - dr) * dc
                  + v10 * dr * (1.0 - dc)
                  + v11 * dr * dc;

            new_data[[r, c]] = v.round() as i16;
        }
    }
    new_data
}

/// Accumulate face normals and normalize to per-vertex normals
fn compute_vertex_normals_local(vertices: &[[f32; 3]], faces: &[[usize; 3]]) -> Vec<[f32; 3]> {
    let mut acc: Vec<[f32; 3]> = vec![[0.0, 0.0, 0.0]; vertices.len()];
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
        if len > 1e-6 { [ a[0]/len, a[1]/len, a[2]/len ] } else { [0.0, 0.0, 1.0] }
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
