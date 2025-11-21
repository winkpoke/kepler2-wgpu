#![allow(dead_code)]

use crate::data::CTVolume;
use std::collections::HashMap;
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
    pub window_scale: f32,
    pub window_offset: f32,
    pub opacity: f32,
    pub _padding2: f32,
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
            window_scale: 1.0,
            window_offset: 0.5,
            opacity: 1.0,
            _padding2: 0.0,
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
            ambient_color: [0.2, 0.2, 0.2],       // Dim ambient light
            ambient_intensity: 0.3,
            window_scale: 1.0,
            window_offset: 0.5,
            opacity: 1.0,
            _padding2: 0.0,
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

/// Function-level comment: Marching Tetrahedra algorithm implementation for isosurface extraction
/// Converts 3D volume data into triangular mesh representation using tetrahedral decomposition
#[derive(Clone, Debug)]
struct MarchingTetrahedra {
    cube_to_tetrahedra: [[usize; 4]; 6],
    tetrahedra_edges: [[usize; 2]; 6],
    triangle_table: HashMap<u8, Vec<[usize; 3]>>,
}

impl MarchingTetrahedra {
    fn new() -> Self {
        let mut triangle_table: HashMap<u8, Vec<[usize; 3]>> = HashMap::new();
        triangle_table.insert(0b0000, vec![]);
        triangle_table.insert(0b0001, vec![[0, 1, 2]]);
        triangle_table.insert(0b0010, vec![[0, 3, 4]]);
        triangle_table.insert(0b0100, vec![[1, 3, 5]]);
        triangle_table.insert(0b1000, vec![[2, 4, 5]]);
        triangle_table.insert(0b0011, vec![[2, 4, 3], [2, 3, 1]]);
        triangle_table.insert(0b0101, vec![[0, 2, 5], [0, 5, 1]]);
        triangle_table.insert(0b0110, vec![[0, 1, 5], [0, 5, 3]]);
        triangle_table.insert(0b1001, vec![[0, 4, 5], [0, 5, 2]]);
        triangle_table.insert(0b1010, vec![[0, 3, 5], [0, 5, 4]]);
        triangle_table.insert(0b1100, vec![[1, 4, 5], [1, 5, 3]]);
        triangle_table.insert(0b1110, vec![[0, 2, 1]]);
        triangle_table.insert(0b1101, vec![[0, 4, 3]]);
        triangle_table.insert(0b1011, vec![[1, 5, 3]]);
        triangle_table.insert(0b0111, vec![[2, 5, 4]]);
        triangle_table.insert(0b1111, vec![]);

        MarchingTetrahedra {
            cube_to_tetrahedra: [
                [0, 1, 3, 4],
                [1, 4, 5, 7],
                [1, 3, 4, 7],
                [1, 2, 3, 7],
                [2, 3, 6, 7],
                [3, 4, 6, 7],
            ],
            tetrahedra_edges: [
                [0, 1], [0, 2], [0, 3],
                [1, 2], [1, 3], [2, 3],
            ],
            triangle_table,
        }
    }

    fn extract_isosurface(
        &self,
        voxel_data: &Vec<i16>,
        dimensions: (usize, usize, usize),
        iso_value: f32,
        spacing: (f32, f32, f32),
        downsample: Option<usize>,
        vertex_precision: usize,
    ) -> (Vec<MeshVertex>, Vec<u16>) {
        log::info!("原始体数据大小: {:?}", dimensions);

        // Convert Vec<i16> to Array3<f32> with correct dimensions
        // Note: dimensions are (width, height, depth) but we need (depth, height, width) for Array3
        let vec_f32: Vec<f32> = voxel_data.iter().map(|&v| v as f32).collect();
        let mut volume = match Array3::from_shape_vec((dimensions.2, dimensions.1, dimensions.0), vec_f32) {
            Ok(arr) => arr,
            Err(e) => {
                log::warn!(
                    "volume shape mismatch: expected {} elements, got {}; error: {}",
                    dimensions.2 * dimensions.1 * dimensions.0,
                    voxel_data.len(),
                    e
                );
                return (vec![], vec![]);
            }
        };
        
        let mut final_spacing = spacing;

        if let Some(ds) = downsample {
            if ds > 1 {
                log::info!("降采样因子: {}", ds);
                let new_shape = [
                    volume.shape()[0] / ds,
                    volume.shape()[1] / ds,
                    volume.shape()[2] / ds,
                ];
                let mut downsampled = Array3::zeros(new_shape);
                for z in 0..new_shape[0] {
                    for y in 0..new_shape[1] {
                        for x in 0..new_shape[2] {
                            let slice = volume.slice(ndarray::s![
                                z * ds..(z + 1) * ds,
                                y * ds..(y + 1) * ds,
                                x * ds..(x + 1) * ds
                            ]);
                            downsampled[[z, y, x]] = slice.mean().unwrap_or(0.0);
                        }
                    }
                }
                volume = downsampled;
                final_spacing = (spacing.0 * ds as f32, spacing.1 * ds as f32, spacing.2 * ds as f32);
                log::info!("降采样后大小: {:?}", volume.shape());
                log::info!("更新后的体素间距: {:?}", final_spacing);
            }
        }

        let shape = volume.shape();
        let depth = shape[0];
        let height = shape[1];
        let width = shape[2];

        log::info!("筛选活跃区域...");
        let active_mask = self.find_active_voxels(&volume, iso_value);
        let mut active_indices: Vec<(usize, usize, usize)> = vec![];
        for z in 0..depth - 1 {
            for y in 0..height - 1 {
                for x in 0..width - 1 {
                    if active_mask[[z, y, x]] {
                        active_indices.push((z, y, x));
                    }
                }
            }
        }
        log::info!("活跃体素数: {}", active_indices.len());

        if active_indices.is_empty() {
            log::warn!("未找到包含等值面的体素");
            return (vec![], vec![]);
        }

        let mut vertex_dict: HashMap<(i32, i32, i32), usize> = HashMap::new();
        let mut vertices: Vec<[f32; 3]> = vec![];
        let mut triangles: Vec<[u32; 3]> = vec![];

        let total = active_indices.len();
        for (idx, &(z, y, x)) in active_indices.iter().enumerate() {
            if idx % 10000 == 0 {
                log::info!("处理进度: {}/{} ({:.1}%)", idx, total, 100.0 * idx as f32 / total as f32);
            }

            let cube_values = [
                volume[[z, y, x]],
                volume[[z, y, x + 1]],
                volume[[z, y + 1, x + 1]],
                volume[[z, y + 1, x]],
                volume[[z + 1, y, x]],
                volume[[z + 1, y, x + 1]],
                volume[[z + 1, y + 1, x + 1]],
                volume[[z + 1, y + 1, x]],
            ];

            let cube_positions = [
                [x as f32 * final_spacing.0, y as f32 * final_spacing.1, z as f32 * final_spacing.2],
                [(x + 1) as f32 * final_spacing.0, y as f32 * final_spacing.1, z as f32 * final_spacing.2],
                [(x + 1) as f32 * final_spacing.0, (y + 1) as f32 * final_spacing.1, z as f32 * final_spacing.2],
                [x as f32 * final_spacing.0, (y + 1) as f32 * final_spacing.1, z as f32 * final_spacing.2],
                [x as f32 * final_spacing.0, y as f32 * final_spacing.1, (z + 1) as f32 * final_spacing.2],
                [(x + 1) as f32 * final_spacing.0, y as f32 * final_spacing.1, (z + 1) as f32 * final_spacing.2],
                [(x + 1) as f32 * final_spacing.0, (y + 1) as f32 * final_spacing.1, (z + 1) as f32 * final_spacing.2],
                [x as f32 * final_spacing.0, (y + 1) as f32 * final_spacing.1, (z + 1) as f32 * final_spacing.2],
            ];

            for &tet_indices in self.cube_to_tetrahedra.iter() {
                let tet_positions = [
                    cube_positions[tet_indices[0]],
                    cube_positions[tet_indices[1]],
                    cube_positions[tet_indices[2]],
                    cube_positions[tet_indices[3]],
                ];
                let tet_values = [
                    cube_values[tet_indices[0]],
                    cube_values[tet_indices[1]],
                    cube_values[tet_indices[2]],
                    cube_values[tet_indices[3]],
                ];
                
                self.process_tetrahedron_dedup(
                    &tet_positions,
                    &tet_values,
                    iso_value,
                    &mut vertices,
                    &mut triangles,
                    &mut vertex_dict,
                    vertex_precision,
                );
            }
        }

        log::info!("生成完成: {} 个顶点, {} 个三角形", vertices.len(), triangles.len());

        if vertices.is_empty() {
            return (vec![], vec![]);
        }

        // // Convert vertices to MeshVertex format with proper coordinate system
        // let mesh_vertices: Vec<MeshVertex> = vertices.iter().map(|&v| {
        //     // Convert from volume coordinates to world coordinates
        //     // Medical imaging typically uses DICOM coordinate system
        //     MeshVertex {
        //         position: [v[0], v[1], v[2]], // Keep original coordinates for now
        //         normal: [0.0, 0.0, 1.0], // Simple normal for now
        //         color: [0.8, 0.8, 0.8], // Default gray color
        //     }
        // }).collect();

        // // Convert triangles to u16 format (flatten the triangle indices)
        // let mesh_triangles: Vec<u16> = triangles.iter().flat_map(|tri| {
        //     tri.iter().map(|&idx| idx as u16)
        // }).collect();

        // 1. 计算包围盒（体素坐标系）
        let mut min_vox = [f32::INFINITY; 3];
        let mut max_vox = [f32::NEG_INFINITY; 3];
        for &v in &vertices {
            for i in 0..3 {
                min_vox[i] = min_vox[i].min(v[i]);
                max_vox[i] = max_vox[i].max(v[i]);
            }
        }
        let center_vox = [
            (min_vox[0] + max_vox[0]) * 0.5,
            (min_vox[1] + max_vox[1]) * 0.5,
            (min_vox[2] + max_vox[2]) * 0.5,
        ];
        let extent_vox = [
            max_vox[0] - min_vox[0],
            max_vox[1] - min_vox[1],
            max_vox[2] - min_vox[2],
        ];
        let max_extent = extent_vox.iter().fold(0.0f32, |a, &b| a.max(b));
        if max_extent < f32::EPSILON {
            log::warn!("extract_isosurface: empty bounding box, skipping.");
            return (vec![], vec![]);
        }

        // 2. 归一化 + 可选地放大到「医学常用 512 体素≈ -256..256 mm」范围
        //    这里直接缩到 [-1,1] 立方体，摄像机放在 z=2 看原点即可看到。
        let scale = 1.0 / max_extent;
        let mut mesh_vertices = Vec::with_capacity(vertices.len());
        for v in vertices {
            let x = (v[0] - center_vox[0]) * scale;
            let y = (v[1] - center_vox[1]) * scale;
            let z = (v[2] - center_vox[2]) * scale;
            // 简单面法线：沿 Z 向上，以后可换加权法线
            let normal = [0.0, 0.0, 1.0];
            // 高对比颜色方便第一眼看到
            let color = [0.95, 0.7, 0.2];
            mesh_vertices.push(MeshVertex {
                position: [x, y, z],
                normal,
                color,
            });
        }

        // 3. 索引转 u16（之前已经 flatten 过，这里直接 cast）
        let mesh_triangles: Vec<u16> = triangles
            .into_iter()
            .flat_map(|tri| tri.into_iter().map(|i| i as u16)) // 使用 `into_iter` 而不是 `iter`
            .collect();

        log::info!(
            "extract_isosurface: {} vertices, {} triangles (normalized to unit cube)",
            mesh_vertices.len(),
            mesh_triangles.len() / 3
        );

        (mesh_vertices, mesh_triangles)
    }

    fn find_active_voxels(&self, volume: &Array3<f32>, iso_value: f32) -> Array3<bool> {
        let shape = volume.shape();
        let depth = shape[0] - 1;
        let height = shape[1] - 1;
        let width = shape[2] - 1;

        let mut mask = Array3::from_elem((depth, height, width), false);

        for z in 0..depth {
            for y in 0..height {
                for x in 0..width {
                    let cube_vals = [
                        volume[[z, y, x]],
                        volume[[z, y, x + 1]],
                        volume[[z, y + 1, x + 1]],
                        volume[[z, y + 1, x]],
                        volume[[z + 1, y, x]],
                        volume[[z + 1, y, x + 1]],
                        volume[[z + 1, y + 1, x + 1]],
                        volume[[z + 1, y + 1, x]],
                    ];
                    let cube_min = cube_vals.iter().fold(f32::INFINITY, |a, &b| a.min(b));
                    let cube_max = cube_vals.iter().fold(f32::NEG_INFINITY, |a, &b| a.max(b));
                    mask[[z, y, x]] = cube_min < iso_value && cube_max >= iso_value;
                }
            }
        }

        mask
    }

    fn process_tetrahedron_dedup(
        &self,
        positions: &[[f32; 3]],
        values: &[f32],
        iso_value: f32,
        vertices: &mut Vec<[f32; 3]>,
        triangles: &mut Vec<[u32; 3]>,
        vertex_dict: &mut HashMap<(i32, i32, i32), usize>,
        precision: usize,
    ) {
        let inside = [
            values[0] >= iso_value,
            values[1] >= iso_value,
            values[2] >= iso_value,
            values[3] >= iso_value,
        ];
        let case_index: u8 = (inside[0] as u8) << 0
            | (inside[1] as u8) << 1
            | (inside[2] as u8) << 2
            | (inside[3] as u8) << 3;

        if case_index == 0 || case_index == 15 {
            return;
        }

        if let Some(triangle_config) = self.triangle_table.get(&case_index) {
            if triangle_config.is_empty() {
                return;
            }

            let mut edge_vertex_indices: Vec<Option<usize>> = vec![None; 6];
            for (i, edge) in self.tetrahedra_edges.iter().enumerate() {
                let v1 = edge[0];
                let v2 = edge[1];
                if inside[v1] != inside[v2] {
                    let val1 = values[v1];
                    let val2 = values[v2];
                    let t = if (val1 - val2).abs() > 1e-10 {
                        (iso_value - val1) / (val2 - val1)
                    } else {
                        0.5
                    };
                    let vertex = [
                        positions[v1][0] + t * (positions[v2][0] - positions[v1][0]),
                        positions[v1][1] + t * (positions[v2][1] - positions[v1][1]),
                        positions[v1][2] + t * (positions[v2][2] - positions[v1][2]),
                    ];

                    let factor = 10f32.powi(precision as i32);
                    let vertex_key = (
                        (vertex[0] * factor).round() as i32,
                        (vertex[1] * factor).round() as i32,
                        (vertex[2] * factor).round() as i32,
                    );

                    let index = if let Some(&idx) = vertex_dict.get(&vertex_key) {
                        idx
                    } else {
                        let idx = vertices.len();
                        vertices.push(vertex);
                        vertex_dict.insert(vertex_key, idx);
                        idx
                    };

                    edge_vertex_indices[i] = Some(index);
                }
            }

            for tri in triangle_config.iter() {
                let mut tri_indices: [u32; 3] = [0; 3];
                let mut valid = true;
                for (j, &edge_idx) in tri.iter().enumerate() {
                    if let Some(idx) = edge_vertex_indices[edge_idx] {
                        tri_indices[j] = idx as u32;
                    } else {
                        valid = false;
                        break;
                    }
                }
                if valid {
                    triangles.push(tri_indices);
                }
            }
        }
    }
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
                base_index + 0, base_index + 1, base_index + 2,  base_index + 2, base_index + 3, base_index + 0,
                // Back face
                base_index + 4, base_index + 5, base_index + 6,  base_index + 6, base_index + 7, base_index + 4,
                // Bottom face
                base_index + 8, base_index + 9, base_index + 10,  base_index + 10, base_index + 11, base_index + 8,
                // Top face
                base_index + 12, base_index + 13, base_index + 14,  base_index + 14, base_index + 15, base_index + 12,
                // Left face
                base_index + 16, base_index + 17, base_index + 18,  base_index + 18, base_index + 19, base_index + 16,
                // Right face
                base_index + 20, base_index + 21, base_index + 22,  base_index + 22, base_index + 23, base_index + 20,
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
        downsample: Option<usize>,
        vertex_precision: usize,
    ) -> Self {
        log::info!("开始从CT体积数据生成网格...");
        log::info!("等值面值: {}", iso_value);
        log::info!("降采样: {:?}", downsample);
        log::info!("顶点精度: {}", vertex_precision);
        log::info!("体积维度: {:?}", ctvolume.dimensions);
        log::info!("体素间距: {:?}", ctvolume.voxel_spacing);
        
        let mt = MarchingTetrahedra::new();
        let (vertices, indices) = mt.extract_isosurface(
            &ctvolume.voxel_data,
            ctvolume.dimensions,
            iso_value,
            ctvolume.voxel_spacing,
            downsample,
            vertex_precision,
        );
        
        if vertices.is_empty() {
            log::warn!("未生成任何网格数据");
            return Self { vertices: Vec::new(), indices: Vec::new() };
        }
        
        // Calculate mesh bounds for debugging
        let mut min_bounds = [f32::INFINITY, f32::INFINITY, f32::INFINITY];
        let mut max_bounds = [f32::NEG_INFINITY, f32::NEG_INFINITY, f32::NEG_INFINITY];
        
        for vertex in &vertices {
            for i in 0..3 {
                min_bounds[i] = min_bounds[i].min(vertex.position[i]);
                max_bounds[i] = max_bounds[i].max(vertex.position[i]);
            }
        }
        
        let triangle_count = indices.len() / 3;
        log::info!("网格生成完成: {} 个顶点, {} 个三角形", vertices.len(), triangle_count);
        
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
