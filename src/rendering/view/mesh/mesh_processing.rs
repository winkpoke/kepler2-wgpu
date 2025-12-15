use std::collections::HashMap;
use ndarray::{Array3, s};
#[cfg(not(target_arch = "wasm32"))]
use rayon::prelude::*;

/// Merge duplicate vertices using sort-based grouping to reduce memory.
pub fn merge_vertices(
    vertices: &[[f32; 3]],
    faces: &[[usize; 3]],
) -> (Vec<[f32; 3]>, Vec<[usize; 3]>) {
    log::debug!("Merging vertices (Sort-based)...");

    // Build sortable list of bit patterns and original indices
    let mut indexed_verts: Vec<([u32; 3], usize)> = vertices
        .iter()
        .enumerate()
        .map(|(i, v)| ([v[0].to_bits(), v[1].to_bits(), v[2].to_bits()], i))
        .collect();

    // Group duplicates by sorting
    indexed_verts.sort_unstable_by_key(|k| k.0);

    let mut unique_vertices: Vec<[f32; 3]> = Vec::new();
    let mut index_map = vec![0usize; vertices.len()];

    if !indexed_verts.is_empty() {
        let first = indexed_verts[0];
        unique_vertices.push(vertices[first.1]);
        index_map[first.1] = 0;

        let mut current_unique_idx = 0usize;
        let mut last_bits = first.0;

        for i in 1..indexed_verts.len() {
            let current = indexed_verts[i];
            if current.0 == last_bits {
                index_map[current.1] = current_unique_idx;
            } else {
                current_unique_idx += 1;
                unique_vertices.push(vertices[current.1]);
                index_map[current.1] = current_unique_idx;
                last_bits = current.0;
            }
        }
    }

    let new_faces: Vec<[usize; 3]> = faces
        .iter()
        .map(|f| [index_map[f[0]], index_map[f[1]], index_map[f[2]]])
        .collect();

    log::debug!(
        "Merged {} -> {} vertices",
        vertices.len(),
        unique_vertices.len()
    );
    (unique_vertices, new_faces)
}

/// Laplacian smoothing using face-based accumulators for low memory.
pub fn laplacian_smooth(
    vertices: &[[f32; 3]],
    faces: &[[usize; 3]],
    iterations: usize,
    lambda: f32,
) -> Vec<[f32; 3]> {
    if vertices.is_empty() || faces.is_empty() {
        return vertices.to_vec();
    }

    log::debug!("Smoothing mesh (Accumulator-based)...");
    let mut current_vertices = vertices.to_vec();
    let num_verts = current_vertices.len();

    let mut moves = vec![[0.0f32; 3]; num_verts];
    let mut counts = vec![0u32; num_verts];

    for _ in 0..iterations {
        moves.fill([0.0, 0.0, 0.0]);
        counts.fill(0);

        for face in faces {
            let ia = face[0];
            let ib = face[1];
            let ic = face[2];
            if ia == ib || ib == ic || ia == ic { continue; }

            let pa = current_vertices[ia];
            let pb = current_vertices[ib];
            let pc = current_vertices[ic];

            moves[ia][0] += pb[0] + pc[0];
            moves[ia][1] += pb[1] + pc[1];
            moves[ia][2] += pb[2] + pc[2];
            counts[ia] += 2;

            moves[ib][0] += pa[0] + pc[0];
            moves[ib][1] += pa[1] + pc[1];
            moves[ib][2] += pa[2] + pc[2];
            counts[ib] += 2;

            moves[ic][0] += pa[0] + pb[0];
            moves[ic][1] += pa[1] + pb[1];
            moves[ic][2] += pa[2] + pb[2];
            counts[ic] += 2;
        }

        for i in 0..num_verts {
            if counts[i] > 0 {
                let scale = 1.0 / counts[i] as f32;
                let center = [
                    moves[i][0] * scale,
                    moves[i][1] * scale,
                    moves[i][2] * scale,
                ];
                let old = current_vertices[i];
                current_vertices[i] = [
                    old[0] + lambda * (center[0] - old[0]),
                    old[1] + lambda * (center[1] - old[1]),
                    old[2] + lambda * (center[2] - old[2]),
                ];
            }
        }
    }

    current_vertices
}

pub struct MarchingTetrahedra {
    threshold: i16,
}

impl MarchingTetrahedra {
    pub fn new(threshold: i16) -> Self {
        Self { threshold }
    }

    /// Interpolate along an edge and return f32 position.
    fn interpolate_vertex(&self, p1: [f64; 3], p2: [f64; 3], v1: i16, v2: i16) -> [f32; 3] {
        let v1 = v1 as f64;
        let v2 = v2 as f64;
        let t = self.threshold as f64;
        let res = if (t - v1).abs() < 1e-5 {
            p1
        } else if (t - v2).abs() < 1e-5 {
            p2
        } else if (v1 - v2).abs() < 1e-5 {
            p1
        } else {
            let mu = (t - v1) / (v2 - v1);
            [
                p1[0] + mu * (p2[0] - p1[0]),
                p1[1] + mu * (p2[1] - p1[1]),
                p1[2] + mu * (p2[2] - p1[2]),
            ]
        };
        [res[0] as f32, res[1] as f32, res[2] as f32]
    }

    /// Extract isosurface using marching tetrahedra, output f32 vertices.
    pub fn extract_surface(&self, volume: &Array3<i16>, spacing: (f64, f64, f64)) -> (Vec<[f32; 3]>, Vec<[usize; 3]>) {
        let (depth, height, width) = volume.dim();
        let (spacing_z, spacing_y, spacing_x) = spacing;
        log::debug!("Processing volume: {}x{}x{}", width, height, depth);
        log::debug!("Spacing: {:.3}x{:.3}x{:.3}", spacing_x, spacing_y, spacing_z);

        // Parallelize over Z slices (native), fallback to sequential on WASM
        #[cfg(not(target_arch = "wasm32"))]
        let all_vertices: Vec<[f32; 3]> = (0..depth - 1).into_par_iter().flat_map(|z| {
            self.process_slice(volume, z, width, height, spacing_x, spacing_y, spacing_z)
        }).collect();

        #[cfg(target_arch = "wasm32")]
        let all_vertices: Vec<[f32; 3]> = (0..depth - 1).into_iter().flat_map(|z| {
            self.process_slice(volume, z, width, height, spacing_x, spacing_y, spacing_z)
        }).collect();

        log::debug!("Generated {} vertices", all_vertices.len());
        
        let num_triangles = all_vertices.len() / 3;
        let faces: Vec<[usize; 3]> = (0..num_triangles)
            .map(|i| [i * 3, i * 3 + 1, i * 3 + 2])
            .collect();

        log::debug!("Generated {} triangles", faces.len());
        
        (all_vertices, faces)
    }

    fn process_slice(
        &self,
        volume: &Array3<i16>,
        z: usize,
        width: usize,
        height: usize,
        spacing_x: f64,
        spacing_y: f64,
        spacing_z: f64,
    ) -> Vec<[f32; 3]> {
        let mut local_vertices = Vec::new();

        for y in 0..height - 1 {
            for x in 0..width - 1 {
                // Get cube values
                let v0 = volume[[z, y, x]];
                let v1 = volume[[z, y, x + 1]];
                let v2 = volume[[z, y + 1, x + 1]];
                let v3 = volume[[z, y + 1, x]];
                let v4 = volume[[z + 1, y, x]];
                let v5 = volume[[z + 1, y, x + 1]];
                let v6 = volume[[z + 1, y + 1, x + 1]];
                let v7 = volume[[z + 1, y + 1, x]];

                let cube_values = [v0, v1, v2, v3, v4, v5, v6, v7];

                // Decompose into 6 tetrahedra
                let tetrahedrons = [
                    [0, 1, 3, 5],
                    [1, 2, 3, 5],
                    [2, 3, 5, 6],
                    [0, 3, 4, 5],
                    [3, 4, 5, 7],
                    [3, 5, 6, 7],
                ];

                for tetra_indices in tetrahedrons {
                    let mut tetra_index = 0;
                    let mut t_values = [0; 4];
                    let mut t_indices = [0; 4];

                    for (i, &idx) in tetra_indices.iter().enumerate() {
                        let val = cube_values[idx];
                        t_values[i] = val;
                        t_indices[i] = idx;
                        if val > self.threshold {
                            tetra_index |= 1 << i;
                        }
                    }

                    if tetra_index == 0 || tetra_index == 15 {
                        continue;
                    }

                    let edges = &TETRA_TABLE[tetra_index];
                    
                    for i in (0..6).step_by(3) {
                        if edges[i] == -1 {
                            break;
                        }

                        let e1 = edges[i] as usize;
                        let e2 = edges[i+1] as usize;
                        let e3 = edges[i+2] as usize;

                        for &edge_idx in &[e1, e2, e3] {
                            // Map edge index (0-5) to vertex pair in tetrahedron
                            let (v1_local, v2_local) = TETRA_EDGE_VERTICES[edge_idx];
                            let v1_cube_idx = t_indices[v1_local];
                            let v2_cube_idx = t_indices[v2_local];
                            
                            let get_pos = |idx| {
                                let dx = if idx == 1 || idx == 2 || idx == 5 || idx == 6 { 1.0 } else { 0.0 };
                                let dy = if idx == 2 || idx == 3 || idx == 6 || idx == 7 { 1.0 } else { 0.0 };
                                let dz = if idx >= 4 { 1.0 } else { 0.0 };
                                [
                                    (x as f64 + dx) * spacing_x,
                                    (y as f64 + dy) * spacing_y,
                                    (z as f64 + dz) * spacing_z
                                ]
                            };

                            let p1 = get_pos(v1_cube_idx);
                            let p2 = get_pos(v2_cube_idx);
                            let val1 = cube_values[v1_cube_idx];
                            let val2 = cube_values[v2_cube_idx];

                            local_vertices.push(self.interpolate_vertex(p1, p2, val1, val2));
                        }
                    }
                }
            }
        }
        local_vertices
    }
}

// Tetrahedron edge definitions (local indices 0-3)
const TETRA_EDGE_VERTICES: [(usize, usize); 6] = [
    (0, 1), // Edge 0
    (1, 2), // Edge 1
    (2, 0), // Edge 2
    (0, 3), // Edge 3
    (1, 3), // Edge 4
    (2, 3), // Edge 5
];

// Lookup table for Marching Tetrahedra
// 16 cases, max 2 triangles (6 indices)
// -1 indicates end of list
const TETRA_TABLE: [[i8; 7]; 16] = [
    [-1, -1, -1, -1, -1, -1, -1], // 0000
    [0, 2, 3, -1, -1, -1, -1],    // 0001
    [0, 1, 4, -1, -1, -1, -1],    // 0010
    [1, 2, 4, 2, 3, 4, -1],       // 0011
    [1, 2, 5, -1, -1, -1, -1],    // 0100
    [0, 3, 5, 0, 5, 1, -1],       // 0101
    [0, 2, 5, 0, 5, 4, -1],       // 0110
    [3, 4, 5, -1, -1, -1, -1],    // 0111
    [3, 4, 5, -1, -1, -1, -1],    // 1000
    [0, 5, 4, 0, 2, 5, -1],       // 1001
    [0, 5, 1, 0, 3, 5, -1],       // 1010
    [1, 2, 5, -1, -1, -1, -1],    // 1011
    [1, 4, 2, 2, 4, 3, -1],       // 1100
    [0, 1, 4, -1, -1, -1, -1],    // 1101
    [0, 2, 3, -1, -1, -1, -1],    // 1110
    [-1, -1, -1, -1, -1, -1, -1], // 1111
];

/// Downsample mesh by grid clustering using f32 for efficiency.
pub fn downsample_mesh(
    vertices: &[[f32; 3]],
    faces: &[[usize; 3]],
    grid_size: f64,
) -> (Vec<[f32; 3]>, Vec<[usize; 3]>) {
    let grid_size = grid_size as f32;
    if vertices.is_empty() || faces.is_empty() || grid_size <= 0.0 {
        return (vertices.to_vec(), faces.to_vec());
    }

    log::debug!("Downsampling mesh...");

    let mut cell_map: HashMap<[i32; 3], Vec<usize>> = HashMap::new();
    for (i, v) in vertices.iter().enumerate() {
        let key = [
            (v[0] / grid_size).floor() as i32,
            (v[1] / grid_size).floor() as i32,
            (v[2] / grid_size).floor() as i32,
        ];
        cell_map.entry(key).or_default().push(i);
    }

    let mut new_vertices = Vec::with_capacity(cell_map.len());
    let mut old_to_new = vec![0; vertices.len()];

    for (new_idx, indices) in cell_map.values().enumerate() {
        let mut sum_pos = [0.0f32, 0.0f32, 0.0f32];
        for &old_idx in indices {
            let p = vertices[old_idx];
            sum_pos[0] += p[0];
            sum_pos[1] += p[1];
            sum_pos[2] += p[2];
        }
        let count = indices.len() as f32;
        let centroid = [
            sum_pos[0] / count,
            sum_pos[1] / count,
            sum_pos[2] / count,
        ];
        new_vertices.push(centroid);

        for &old_idx in indices {
            old_to_new[old_idx] = new_idx;
        }
    }

    let mut new_faces = Vec::with_capacity(faces.len());
    for face in faces {
        let v0 = old_to_new[face[0]];
        let v1 = old_to_new[face[1]];
        let v2 = old_to_new[face[2]];
        if v0 != v1 && v1 != v2 && v0 != v2 {
            new_faces.push([v0, v1, v2]);
        }
    }

    (new_vertices, new_faces)
}

/// Applies a Gaussian filter to a 3D volume.
/// 
/// # Arguments
/// * `volume` - The input 3D volume (i16).
/// * `sigma` - The standard deviation of the Gaussian kernel.
/// 
/// # Returns
/// A new 3D volume with the filter applied.
pub fn apply_gaussian_filter(volume: &Array3<i16>, sigma: f64) -> Array3<i16> {
    if sigma <= 0.0 {
        return volume.clone();
    }

    let kernel = create_gaussian_kernel(sigma);
    let (depth, rows, cols) = volume.dim();
    
    // Convert to f64 for processing
    let mut data_f64 = volume.mapv(|v| v as f64);
    let mut temp = Array3::<f64>::zeros((depth, rows, cols));

    // Convolve along Z
    for y in 0..rows {
        for x in 0..cols {
            let line = data_f64.slice(s![.., y, x]);
            let convolved = convolve_1d(&line.to_vec(), &kernel);
            for (z, val) in convolved.iter().enumerate() {
                temp[[z, y, x]] = *val;
            }
        }
    }
    data_f64.assign(&temp);

    // Convolve along Y
    for z in 0..depth {
        for x in 0..cols {
            let line = data_f64.slice(s![z, .., x]);
            let convolved = convolve_1d(&line.to_vec(), &kernel);
            for (y, val) in convolved.iter().enumerate() {
                temp[[z, y, x]] = *val;
            }
        }
    }
    data_f64.assign(&temp);

    // Convolve along X
    for z in 0..depth {
        for y in 0..rows {
            let line = data_f64.slice(s![z, y, ..]);
            let convolved = convolve_1d(&line.to_vec(), &kernel);
            for (x, val) in convolved.iter().enumerate() {
                temp[[z, y, x]] = *val;
            }
        }
    }

    // Convert back to i16
    temp.mapv(|v| v.round() as i16)
}

fn create_gaussian_kernel(sigma: f64) -> Vec<f64> {
    let radius = (3.0 * sigma).ceil() as isize;
    let size = (2 * radius + 1) as usize;
    let mut kernel = Vec::with_capacity(size);
    let mut sum = 0.0;
    let two_sigma_sq = 2.0 * sigma * sigma;
    // Removed unused 'norm' variable and PI import

    for i in -radius..=radius {
        let x = i as f64;
        let val = (-x * x / two_sigma_sq).exp();
        kernel.push(val);
        sum += val;
    }

    // Normalize
    for val in &mut kernel {
        *val /= sum;
    }

    kernel
}

fn convolve_1d(data: &[f64], kernel: &[f64]) -> Vec<f64> {
    let len = data.len();
    let k_len = kernel.len();
    let radius = k_len / 2;
    let mut result = vec![0.0; len];

    for i in 0..len {
        let mut sum = 0.0;
        for j in 0..k_len {
            let k_idx = j as isize - radius as isize;
            let d_idx = i as isize + k_idx;
            
            // Mirror boundary conditions
            let val = if d_idx < 0 {
                data[(-d_idx) as usize]
            } else if d_idx >= len as isize {
                data[(2 * len as isize - d_idx - 2) as usize] // Mirror at boundary
            } else {
                data[d_idx as usize]
            };
            
            sum += val * kernel[j];
        }
        result[i] = sum;
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use ndarray::arr3;

    #[test]
    fn test_gaussian_kernel_sum() {
        let kernel = create_gaussian_kernel(1.0);
        let sum: f64 = kernel.iter().sum();
        assert!((sum - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_apply_gaussian_filter_identity() {
        let volume = arr3(&[
            [[10, 10], [10, 10]],
            [[10, 10], [10, 10]]
        ]);
        // Sigma 0 should return original
        let filtered = apply_gaussian_filter(&volume, 0.0);
        assert_eq!(filtered, volume);
    }
}
