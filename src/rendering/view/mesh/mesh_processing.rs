use ndarray::{s, Array3, ArrayView3};
#[cfg(not(target_arch = "wasm32"))]
use rayon::prelude::*;
use std::collections::HashMap;
use std::f64::consts::PI;

// --- 1. Optimized Merge Vertices (Sort-based, Low Memory) ---
pub fn merge_vertices(
    vertices: &[[f32; 3]],
    faces: &[[usize; 3]],
) -> (Vec<[f32; 3]>, Vec<[usize; 3]>) {
    println!("Merging vertices (Sort-based)...");

    // Create indices 0..N
    let mut indices: Vec<usize> = (0..vertices.len()).collect();

    // Sort indices based on vertex values
    // usage: 8 bytes/vertex instead of 20 bytes/vertex
    indices.sort_unstable_by(|&a, &b| {
        let va = vertices[a];
        let vb = vertices[b];
        // Compare bits to handle floats safely
        let ka = [va[0].to_bits(), va[1].to_bits(), va[2].to_bits()];
        let kb = [vb[0].to_bits(), vb[1].to_bits(), vb[2].to_bits()];
        ka.cmp(&kb)
    });

    let mut unique_vertices = Vec::new();
    // Map from old_index -> new_index
    let mut index_map = vec![0usize; vertices.len()];

    if !indices.is_empty() {
        // First one
        let first_idx = indices[0];
        unique_vertices.push(vertices[first_idx]);
        index_map[first_idx] = 0;

        let mut current_unique_idx = 0;
        let v0 = vertices[first_idx];
        let mut last_bits = [v0[0].to_bits(), v0[1].to_bits(), v0[2].to_bits()];

        for &orig_idx in &indices[1..] {
            let v = vertices[orig_idx];
            let current_bits = [v[0].to_bits(), v[1].to_bits(), v[2].to_bits()];

            if current_bits == last_bits {
                // Duplicate
                index_map[orig_idx] = current_unique_idx;
            } else {
                // New
                current_unique_idx += 1;
                unique_vertices.push(v);
                index_map[orig_idx] = current_unique_idx;
                last_bits = current_bits;
            }
        }
    }

    // Rebuild faces
    let new_faces: Vec<[usize; 3]> = faces
        .iter()
        .map(|f| [index_map[f[0]], index_map[f[1]], index_map[f[2]]])
        .collect();

    println!(
        "Merged {} -> {} vertices",
        vertices.len(),
        unique_vertices.len()
    );
    (unique_vertices, new_faces)
}

// --- 2. Optimized Laplacian Smooth (Accumulator-based, Low Memory) ---
pub fn laplacian_smooth(
    vertices: &[[f32; 3]],
    faces: &[[usize; 3]],
    iterations: usize,
    lambda: f32,
    pinned_zs: &[f32], // New argument
) -> Vec<[f32; 3]> {
    if vertices.is_empty() || faces.is_empty() {
        return vertices.to_vec();
    }

    println!("Smoothing mesh (Accumulator-based)...");
    let mut current_vertices = vertices.to_vec();
    let num_verts = current_vertices.len();

    // Pre-calculate which vertices are pinned
    // Using a simple bool mapping might be faster than checking Z every iter
    let is_pinned: Vec<bool> = current_vertices
        .iter()
        .map(|v| pinned_zs.iter().any(|&z_pin| (v[2] - z_pin).abs() < 1e-3))
        .collect();

    // Pre-allocate buffers once
    let mut moves = vec![[0.0f32; 3]; num_verts];
    let mut counts = vec![0u32; num_verts];

    for _ in 0..iterations {
        // Reset buffers
        moves.fill([0.0, 0.0, 0.0]);
        counts.fill(0);

        // Iterate FACES to find neighbors implicitly
        for face in faces {
            let ia = face[0];
            let ib = face[1];
            let ic = face[2];

            // Ignore degenerate triangles within index bounds
            if ia == ib || ib == ic || ia == ic {
                continue;
            }

            let pa = current_vertices[ia];
            let pb = current_vertices[ib];
            let pc = current_vertices[ic];

            // Add pb and pc to a's accumulator
            moves[ia][0] += pb[0] + pc[0];
            moves[ia][1] += pb[1] + pc[1];
            moves[ia][2] += pb[2] + pc[2];
            counts[ia] += 2;

            // Add pa and pc to b's accumulator
            moves[ib][0] += pa[0] + pc[0];
            moves[ib][1] += pa[1] + pc[1];
            moves[ib][2] += pa[2] + pc[2];
            counts[ib] += 2;

            // Add pa and pb to c's accumulator
            moves[ic][0] += pa[0] + pb[0];
            moves[ic][1] += pa[1] + pb[1];
            moves[ic][2] += pa[2] + pb[2];
            counts[ic] += 2;
        }

        // Apply Smoothing
        for i in 0..num_verts {
            // SKIP pinned vertices
            if is_pinned[i] {
                continue;
            }

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

pub fn downsample_mesh(
    vertices: &[[f32; 3]],
    faces: &[[usize; 3]],
    grid_size: f64,
    pinned_zs: &[f32],
) -> (Vec<[f32; 3]>, Vec<[usize; 3]>) {
    let grid_size = grid_size as f32;
    if vertices.is_empty() || faces.is_empty() {
        return (vertices.to_vec(), faces.to_vec());
    }
    if grid_size <= 1e-6 {
        return (vertices.to_vec(), faces.to_vec());
    }

    println!("Downsampling mesh (with pinning)...");

    let is_pinned: Vec<bool> = vertices
        .iter()
        .map(|v| pinned_zs.iter().any(|&z_pin| (v[2] - z_pin).abs() < 1e-3))
        .collect();

    let mut cell_map: HashMap<[i32; 3], Vec<usize>> = HashMap::new();
    let mut new_vertices = Vec::new();
    let mut old_to_new = vec![usize::MAX; vertices.len()];

    // 1. Handle Pinned Vertices
    for (i, v) in vertices.iter().enumerate() {
        if is_pinned[i] {
            old_to_new[i] = new_vertices.len();
            new_vertices.push(*v);
        }
    }

    // 2. Group Unpinned Vertices
    for (i, v) in vertices.iter().enumerate() {
        if !is_pinned[i] {
            let key = [
                (v[0] / grid_size).floor() as i32,
                (v[1] / grid_size).floor() as i32,
                (v[2] / grid_size).floor() as i32,
            ];
            cell_map.entry(key).or_default().push(i);
        }
    }

    // 3. Create Centroids
    for indices in cell_map.values() {
        let mut sum_pos = [0.0, 0.0, 0.0];
        for &old_idx in indices {
            let p = vertices[old_idx];
            sum_pos[0] += p[0];
            sum_pos[1] += p[1];
            sum_pos[2] += p[2];
        }
        let count = indices.len() as f32;
        let centroid = [sum_pos[0] / count, sum_pos[1] / count, sum_pos[2] / count];

        let new_idx = new_vertices.len();
        new_vertices.push(centroid);

        for &old_idx in indices {
            old_to_new[old_idx] = new_idx;
        }
    }

    // 4. Rebuild Faces
    let mut new_faces = Vec::with_capacity(faces.len());
    for face in faces {
        let idx0 = old_to_new[face[0]];
        let idx1 = old_to_new[face[1]];
        let idx2 = old_to_new[face[2]];

        if idx0 != usize::MAX && idx1 != usize::MAX && idx2 != usize::MAX {
            if idx0 != idx1 && idx1 != idx2 && idx0 != idx2 {
                new_faces.push([idx0, idx1, idx2]);
            }
        }
    }

    (new_vertices, new_faces)
}

pub struct MarchingTetrahedra {
    min: i16,
    max: i16,
}

impl MarchingTetrahedra {
    pub fn new(min: i16, max: i16) -> Self {
        Self { min, max }
    }

    /// Interpolate along an edge and return f32 position.
    fn interpolate_vertex(
        &self,
        p1: [f64; 3],
        p2: [f64; 3],
        v1: i16,
        v2: i16,
        target_threshold: i16,
    ) -> [f32; 3] {
        let v1 = v1 as f64;
        let v2 = v2 as f64;
        let t = target_threshold as f64;

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
    pub fn extract_surface(
        &self,
        volume: &ndarray::ArrayView3<i16>,
        spacing: (f64, f64, f64),
        offset: [usize; 3],
        z_range: std::ops::Range<usize>,
    ) -> (Vec<[f32; 3]>, Vec<[usize; 3]>) {
        let (depth, height, width) = volume.dim();
        let (spacing_z, spacing_y, spacing_x) = spacing;

        // Ensure range is within bounds
        let start_z = z_range.start.max(0);
        let end_z = z_range.end.min(depth - 1);

        // Parallelize over Z slices (native), fallback to sequential on WASM
        #[cfg(not(target_arch = "wasm32"))]
        let all_vertices: Vec<[f32; 3]> = (start_z..end_z)
            .into_par_iter()
            .flat_map(|z| {
                self.process_slice(
                    volume, z, width, height, spacing_x, spacing_y, spacing_z, offset,
                )
            })
            .collect();

        #[cfg(target_arch = "wasm32")]
        let all_vertices: Vec<[f32; 3]> = (start_z..end_z)
            .into_iter()
            .flat_map(|z| {
                self.process_slice(
                    volume, z, width, height, spacing_x, spacing_y, spacing_z, offset,
                )
            })
            .collect();

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
        volume: &ArrayView3<i16>,
        z: usize,
        width: usize,
        height: usize,
        spacing_x: f64,
        spacing_y: f64,
        spacing_z: f64,
        offset: [usize; 3],
    ) -> Vec<[f32; 3]> {
        let mut local_vertices = Vec::new();

        for y in 0..height - 1 {
            for x in 0..width - 1 {
                let v0 = volume[[z, y, x]];
                let v1 = volume[[z, y, x + 1]];
                let v2 = volume[[z, y + 1, x + 1]];
                let v3 = volume[[z, y + 1, x]];
                let v4 = volume[[z + 1, y, x]];
                let v5 = volume[[z + 1, y, x + 1]];
                let v6 = volume[[z + 1, y + 1, x + 1]];
                let v7 = volume[[z + 1, y + 1, x]];

                let cube_values = [v0, v1, v2, v3, v4, v5, v6, v7];

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
                    let mut t_indices = [0; 4];

                    for (i, &idx) in tetra_indices.iter().enumerate() {
                        let val = cube_values[idx];
                        t_indices[i] = idx;
                        if val >= self.min && val <= self.max {
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
                        let e2 = edges[i + 1] as usize;
                        let e3 = edges[i + 2] as usize;

                        for &edge_idx in &[e1, e2, e3] {
                            let (v1_local, v2_local) = TETRA_EDGE_VERTICES[edge_idx];
                            let v1_cube_idx = t_indices[v1_local];
                            let v2_cube_idx = t_indices[v2_local];

                            let get_pos = |idx| {
                                let dx = if idx == 1 || idx == 2 || idx == 5 || idx == 6 {
                                    1.0
                                } else {
                                    0.0
                                };
                                let dy = if idx == 2 || idx == 3 || idx == 6 || idx == 7 {
                                    1.0
                                } else {
                                    0.0
                                };
                                let dz = if idx >= 4 { 1.0 } else { 0.0 };
                                [
                                    (x as f64 + offset[0] as f64 + dx) * spacing_x,
                                    (y as f64 + offset[1] as f64 + dy) * spacing_y,
                                    (z as f64 + offset[2] as f64 + dz) * spacing_z,
                                ]
                            };

                            let p1 = get_pos(v1_cube_idx);
                            let p2 = get_pos(v2_cube_idx);
                            let val1 = cube_values[v1_cube_idx];
                            let val2 = cube_values[v2_cube_idx];

                            // Determine which threshold to use
                            // We know one is inside and one is outside because we are processing an edge from TETRA_TABLE
                            let is_v1_inside = val1 >= self.min && val1 <= self.max;
                            let target_threshold = if is_v1_inside {
                                // v1 inside, v2 outside
                                if val2 < self.min {
                                    self.min
                                } else {
                                    self.max
                                }
                            } else {
                                // v2 inside, v1 outside
                                if val1 < self.min {
                                    self.min
                                } else {
                                    self.max
                                }
                            };

                            local_vertices.push(self.interpolate_vertex(
                                p1,
                                p2,
                                val1,
                                val2,
                                target_threshold,
                            ));
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

/// Applies a Gaussian filter to a 3D volume.
///
/// # Arguments
/// * `volume` - The input 3D volume (i16).
/// * `sigma` - The standard deviation of the Gaussian kernel.
///
/// # Returns
/// A new 3D volume with the filter applied.
pub fn apply_gaussian_filter(volume: Array3<i16>, sigma: f32) -> Array3<i16> {
    if sigma <= 0.0 {
        return volume;
    }

    let kernel = create_gaussian_kernel(sigma);
    let (depth, rows, cols) = volume.dim();

    // 1. Convert to f32 and FREE original i16 memory immediately
    // usage: ~700MB (f32) instead of ~350MB(i16) + ~700MB(f32)
    let mut current_buffer = volume.mapv(|v| v as f32);
    // Explicitly drop input if mapv didn't consume it (mapv consumes self for Array, but we want to be sure)
    // Actually mapv on Array3 consumes self, so `volume` is gone.

    // 2. Allocate second buffer for ping-pong
    // usage: +700MB -> Total Peak ~1.4GB
    let mut next_buffer = Array3::<f32>::zeros((depth, rows, cols));

    // Convolve along Z: current -> next
    for y in 0..rows {
        for x in 0..cols {
            let line = current_buffer.slice(s![.., y, x]);
            let convolved = convolve_1d(&line.to_vec(), &kernel);
            for (z, val) in convolved.iter().enumerate() {
                next_buffer[[z, y, x]] = *val;
            }
        }
    }
    // Swap: next is now current
    std::mem::swap(&mut current_buffer, &mut next_buffer);

    // Convolve along Y: current -> next
    for z in 0..depth {
        for x in 0..cols {
            let line = current_buffer.slice(s![z, .., x]);
            let convolved = convolve_1d(&line.to_vec(), &kernel);
            for (y, val) in convolved.iter().enumerate() {
                next_buffer[[z, y, x]] = *val;
            }
        }
    }
    std::mem::swap(&mut current_buffer, &mut next_buffer);

    // Convolve along X: current -> next
    for z in 0..depth {
        for y in 0..rows {
            let line = current_buffer.slice(s![z, y, ..]);
            let convolved = convolve_1d(&line.to_vec(), &kernel);
            for (x, val) in convolved.iter().enumerate() {
                next_buffer[[z, y, x]] = *val;
            }
        }
    }
    std::mem::swap(&mut current_buffer, &mut next_buffer);

    // Final result is in current_buffer. Convert back to i16 and return.
    // We can mapv over it.
    current_buffer.mapv(|v| v.round() as i16)
}

fn create_gaussian_kernel(sigma: f32) -> Vec<f32> {
    let radius = (3.0 * sigma).ceil() as isize;
    let size = (2 * radius + 1) as usize;
    let mut kernel = Vec::with_capacity(size);
    let mut sum = 0.0;
    let two_sigma_sq = 2.0 * sigma * sigma;
    let norm = 1.0 / (2.0 * PI as f32 * sigma * sigma).sqrt(); // 1D normalization is actually just 1/(sqrt(2pi)*sigma)

    for i in -radius..=radius {
        let x = i as f32;
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

fn convolve_1d(data: &[f32], kernel: &[f32]) -> Vec<f32> {
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
    use ndarray::Array3;

    #[test]
    fn test_downsample_pinning() {
        let vertices = vec![
            [0.0, 0.0, 0.0],
            [0.1, 0.1, 0.0],
            [0.0, 0.0, 5.0],
            [0.1, 0.1, 5.0],
        ];
        let faces = vec![[0, 1, 2]];
        let pinned_zs = vec![0.0];
        let grid_size = 1.0;

        let (new_verts, _) = downsample_mesh(&vertices, &faces, grid_size, &pinned_zs);

        assert_eq!(new_verts.len(), 3);
        assert!(new_verts.contains(&[0.0, 0.0, 0.0]));
        assert!(new_verts.contains(&[0.1, 0.1, 0.0]));
    }

    #[test]
    fn test_gaussian_kernel_sum() {
        let kernel = create_gaussian_kernel(1.0);
        let sum: f32 = kernel.iter().sum();
        assert!((sum - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_apply_gaussian_filter_identity() {
        let volume = arr3(&[[[10, 10], [10, 10]], [[10, 10], [10, 10]]]);
        // Sigma 0 should return original
        let filtered = apply_gaussian_filter(volume.clone(), 0.0);
        assert_eq!(filtered, volume);
    }

    #[test]
    fn test_range_extraction() {
        // Create 3x3x3 volume
        let mut volume = Array3::<i16>::zeros((3, 3, 3));

        // Center voxel
        volume[[1, 1, 1]] = 150;

        // Spacing
        let spacing = (1.0, 1.0, 1.0);

        // Case 1: Range [100, 200]
        // Center 150 is INSIDE.
        // Neighbors 0 are OUTSIDE (Low).
        // Should generate surface at threshold 100.
        let mt = MarchingTetrahedra::new(100, 200);
        let (verts, _faces) = mt.extract_surface(&volume.view(), spacing, [0, 0, 0], 0..3);
        assert!(!verts.is_empty(), "Should generate vertices for [100, 200]");

        // Case 2: Range [160, 200]
        // Center 150 is OUTSIDE (Low).
        // Neighbors 0 are OUTSIDE (Low).
        // Should be empty.
        let mt_high = MarchingTetrahedra::new(160, 200);
        let (verts_high, _faces_high) =
            mt_high.extract_surface(&volume.view(), spacing, [0, 0, 0], 0..3);
        assert!(
            verts_high.is_empty(),
            "Should be empty for [160, 200], got {}",
            verts_high.len()
        );

        // Case 3: Range [0, 100]
        // Center 150 is OUTSIDE (High).
        // Neighbors 0 are INSIDE.
        // Should generate surface.
        // But 0 is exactly min. 0 >= 0 && 0 <= 100. So 0 is INSIDE.
        let mt_low = MarchingTetrahedra::new(0, 100);
        let (verts_low, _faces_low) =
            mt_low.extract_surface(&volume.view(), spacing, [0, 0, 0], 0..3);
        assert!(
            !verts_low.is_empty(),
            "Should generate vertices for [0, 100]"
        );

        // Case 4: Range [10, 140]
        // Center 150 OUTSIDE (High).
        // Neighbors 0 OUTSIDE (Low).
        // Should be empty (double crossing issue/feature).
        let mt_split = MarchingTetrahedra::new(10, 140);
        let (verts_split, _faces_split) =
            mt_split.extract_surface(&volume.view(), spacing, [0, 0, 0], 0..3);
        assert!(
            verts_split.is_empty(),
            "Should be empty for split range [10, 140]"
        );
    }
}
