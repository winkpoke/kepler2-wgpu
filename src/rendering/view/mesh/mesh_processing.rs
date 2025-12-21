use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};
use ndarray::Array3;
#[cfg(not(target_arch = "wasm32"))]
use rayon::prelude::*;

// Helper for float hashing
#[derive(Clone, Copy, Debug)]
struct FloatKey([u64; 3]);

impl FloatKey {
    fn new(v: [f64; 3]) -> Self {
        // Quantize to avoid precision issues, but keep enough precision
        // 1e-5 precision is usually enough for MC
        let x = (v[0] * 100000.0) as u64;
        let y = (v[1] * 100000.0) as u64;
        let z = (v[2] * 100000.0) as u64;
        FloatKey([x, y, z])
    }
}

impl PartialEq for FloatKey {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl Eq for FloatKey {}

impl Hash for FloatKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}

pub fn merge_vertices(
    vertices: &[[f64; 3]],
    faces: &[[usize; 3]],
) -> (Vec<[f64; 3]>, Vec<[usize; 3]>) {
    println!("Merging vertices...");
    let mut unique_map: HashMap<FloatKey, usize> = HashMap::new();
    let mut new_vertices: Vec<[f64; 3]> = Vec::new();
    let mut new_faces: Vec<[usize; 3]> = Vec::with_capacity(faces.len());

    for face in faces {
        let mut new_face = [0; 3];
        for i in 0..3 {
            let old_idx = face[i];
            let v = vertices[old_idx];
            let key = FloatKey::new(v);

            let new_idx = *unique_map.entry(key).or_insert_with(|| {
                let idx = new_vertices.len();
                new_vertices.push(v);
                idx
            });
            new_face[i] = new_idx;
        }
        new_faces.push(new_face);
    }

    println!("Merged {} vertices into {} unique vertices", vertices.len(), new_vertices.len());
    (new_vertices, new_faces)
}

pub fn laplacian_smooth(
    vertices: &[[f64; 3]],
    faces: &[[usize; 3]],
    iterations: usize,
    lambda: f64,
) -> Vec<[f64; 3]> {
    if vertices.is_empty() || faces.is_empty() {
        return vertices.to_vec();
    }

    println!("Smoothing mesh: {} vertices, {} iterations", vertices.len(), iterations);

    // 1. Build Adjacency List
    // We use a Vec of HashSets to store unique neighbors for each vertex
    let mut adjacency: Vec<HashSet<usize>> = vec![HashSet::new(); vertices.len()];

    for face in faces {
        let v0 = face[0];
        let v1 = face[1];
        let v2 = face[2];

        // Skip degenerate triangles
        if v0 == v1 || v1 == v2 || v0 == v2 {
            continue;
        }

        adjacency[v0].insert(v1);
        adjacency[v0].insert(v2);

        adjacency[v1].insert(v0);
        adjacency[v1].insert(v2);

        adjacency[v2].insert(v0);
        adjacency[v2].insert(v1);
    }

    // 2. Smoothing Iterations
    let mut current_vertices = vertices.to_vec();
    let mut next_vertices = current_vertices.clone();

    for _iter in 0..iterations {
        for i in 0..current_vertices.len() {
            let neighbors = &adjacency[i];
            if neighbors.is_empty() {
                continue;
            }

            let mut sum_pos = [0.0, 0.0, 0.0];
            for &neighbor_idx in neighbors {
                let neighbor_pos = current_vertices[neighbor_idx];
                sum_pos[0] += neighbor_pos[0];
                sum_pos[1] += neighbor_pos[1];
                sum_pos[2] += neighbor_pos[2];
            }

            let count = neighbors.len() as f64;
            let centroid = [
                sum_pos[0] / count,
                sum_pos[1] / count,
                sum_pos[2] / count,
            ];

            let old_pos = current_vertices[i];
            next_vertices[i] = [
                old_pos[0] + lambda * (centroid[0] - old_pos[0]),
                old_pos[1] + lambda * (centroid[1] - old_pos[1]),
                old_pos[2] + lambda * (centroid[2] - old_pos[2]),
            ];
        }
        
        // Swap buffers
        current_vertices.copy_from_slice(&next_vertices);
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

    fn interpolate_vertex(&self, p1: [f64; 3], p2: [f64; 3], v1: i16, v2: i16) -> [f64; 3] {
        let v1 = v1 as f64;
        let v2 = v2 as f64;
        let t = self.threshold as f64;

        if (t - v1).abs() < 1e-5 {
            return p1;
        }
        if (t - v2).abs() < 1e-5 {
            return p2;
        }
        if (v1 - v2).abs() < 1e-5 {
            return p1;
        }

        let mu = (t - v1) / (v2 - v1);
        [
            p1[0] + mu * (p2[0] - p1[0]),
            p1[1] + mu * (p2[1] - p1[1]),
            p1[2] + mu * (p2[2] - p1[2]),
        ]
    }

    pub fn extract_surface(&self, volume: &Array3<i16>, spacing: (f64, f64, f64)) -> (Vec<[f64; 3]>, Vec<[usize; 3]>) {
        let (depth, height, width) = volume.dim();
        let (spacing_z, spacing_y, spacing_x) = spacing;

        println!("Processing volume: {}x{}x{}", width, height, depth);
        println!("Spacing: {:.3}x{:.3}x{:.3}", spacing_x, spacing_y, spacing_z);

        // Parallelize over Z slices (native), fallback to sequential on WASM
        #[cfg(not(target_arch = "wasm32"))]
        let all_vertices: Vec<[f64; 3]> = (0..depth - 1).into_par_iter().flat_map(|z| {
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
                    // T0: 0, 1, 3, 5
                    // T1: 1, 2, 3, 5
                    // T2: 2, 3, 5, 6
                    // T3: 0, 3, 4, 5
                    // T4: 3, 4, 5, 7
                    // T5: 3, 5, 6, 7
                    
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
        }).collect();

        #[cfg(target_arch = "wasm32")]
        let all_vertices: Vec<[f64; 3]> = (0..depth - 1).into_iter().flat_map(|z| {
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
                            if edges[i] == -1 { break; }

                            let e1 = edges[i] as usize;
                            let e2 = edges[i+1] as usize;
                            let e3 = edges[i+2] as usize;

                            for &edge_idx in &[e1, e2, e3] {
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
        }).collect();

        println!("Generated {} vertices", all_vertices.len());
        
        let num_triangles = all_vertices.len() / 3;
        let faces: Vec<[usize; 3]> = (0..num_triangles)
            .map(|i| [i * 3, i * 3 + 1, i * 3 + 2])
            .collect();

        println!("Generated {} triangles", faces.len());

        (all_vertices, faces)
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
    [0, 2, 3, -1, -1, -1, -1],    // 0001 (v0 in) -> Cut edges connected to v0: (0,1), (0,2), (0,3) -> 0, 2, 3
    [0, 1, 4, -1, -1, -1, -1],    // 0010 (v1 in) -> Cut edges connected to v1: (1,0), (1,2), (1,3) -> 0, 1, 4
    [1, 2, 4, 2, 3, 4, -1],       // 0011 (v0, v1 in) -> Cut (0,2), (0,3), (1,2), (1,3) -> 2, 3, 1, 4. Quad split?
                                  // v0,v1 in. Edges crossing: (0,2), (0,3), (1,2), (1,3). Indices: 2, 3, 1, 4.
                                  // Triangles: (1, 2, 4) and (2, 3, 4) seems reasonable.
    [1, 2, 5, -1, -1, -1, -1],    // 0100 (v2 in) -> Cut (2,0), (2,1), (2,3) -> 2, 1, 5
    [0, 3, 5, 0, 5, 1, -1],       // 0101 (v0, v2 in) -> Cut (0,1), (0,3), (2,1), (2,3) -> 0, 3, 1, 5.
                                  // Triangles: (0, 3, 5) and (0, 5, 1)
    [0, 2, 5, 0, 5, 4, -1],       // 0110 (v1, v2 in) -> Cut (1,0), (1,3), (2,0), (2,3) -> 0, 4, 2, 5.
                                  // Triangles: (0, 2, 5) and (0, 5, 4)
    [3, 4, 5, -1, -1, -1, -1],    // 0111 (v0,v1,v2 in) -> v3 out. Cut (3,0), (3,1), (3,2) -> 3, 4, 5.
    [3, 4, 5, -1, -1, -1, -1],    // 1000 (v3 in) -> Cut (3,0), (3,1), (3,2) -> 3, 4, 5.
    [0, 5, 4, 0, 2, 5, -1],       // 1001 (v0, v3 in) -> v1, v2 out. Same as 0110 inverse?
                                  // v0, v3 in. Cut (0,1), (0,2), (3,1), (3,2) -> 0, 2, 4, 5.
                                  // Triangles: (0, 5, 4) and (0, 2, 5)
    [0, 5, 1, 0, 3, 5, -1],       // 1010 (v1, v3 in) -> v0, v2 out. Same as 0101 inverse?
                                  // v1, v3 in. Cut (1,0), (1,2), (3,0), (3,2) -> 0, 1, 3, 5.
                                  // Triangles: (0, 5, 1) and (0, 3, 5)
    [1, 2, 5, -1, -1, -1, -1],    // 1011 (v0,v1,v3 in) -> v2 out. Cut (2,0), (2,1), (2,3) -> 2, 1, 5.
    [1, 4, 2, 2, 4, 3, -1],       // 1100 (v2, v3 in) -> v0, v1 out. Same as 0011 inverse?
                                  // v2, v3 in. Cut (2,0), (2,1), (3,0), (3,1) -> 2, 1, 3, 4.
                                  // Triangles: (1, 4, 2) and (2, 4, 3)
    [0, 1, 4, -1, -1, -1, -1],    // 1101 (v0,v2,v3 in) -> v1 out. Cut (1,0), (1,2), (1,3) -> 0, 1, 4.
    [0, 2, 3, -1, -1, -1, -1],    // 1110 (v1,v2,v3 in) -> v0 out. Cut (0,1), (0,2), (0,3) -> 0, 2, 3.
    [-1, -1, -1, -1, -1, -1, -1], // 1111
];

pub fn downsample_mesh(
    vertices: &[[f64; 3]],
    faces: &[[usize; 3]],
    grid_size: f64,
) -> (Vec<[f64; 3]>, Vec<[usize; 3]>) {
    if vertices.is_empty() || faces.is_empty() || grid_size <= 0.0 {
        return (vertices.to_vec(), faces.to_vec());
    }

    println!("Downsampling mesh (grid_size={})...", grid_size);

    // 1. Grid Clustering
    // Map each vertex to a grid cell key
    // We use a HashMap to group vertices by cell
    let mut cell_map: HashMap<[i64; 3], Vec<usize>> = HashMap::new();

    for (i, v) in vertices.iter().enumerate() {
        let key = [
            (v[0] / grid_size).floor() as i64,
            (v[1] / grid_size).floor() as i64,
            (v[2] / grid_size).floor() as i64,
        ];
        cell_map.entry(key).or_default().push(i);
    }

    // 2. Compute Representative Vertices
    // For each cell, compute the centroid of its vertices
    let mut new_vertices = Vec::with_capacity(cell_map.len());
    // Map old vertex index to new vertex index
    let mut old_to_new = vec![0; vertices.len()];

    for (new_idx, indices) in cell_map.values().enumerate() {
        let mut sum_pos = [0.0, 0.0, 0.0];
        for &old_idx in indices {
            let p = vertices[old_idx];
            sum_pos[0] += p[0];
            sum_pos[1] += p[1];
            sum_pos[2] += p[2];
        }
        let count = indices.len() as f64;
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

    // 3. Reconstruct Faces
    let mut new_faces = Vec::with_capacity(faces.len());
    for face in faces {
        let v0 = old_to_new[face[0]];
        let v1 = old_to_new[face[1]];
        let v2 = old_to_new[face[2]];

        // Discard degenerate triangles (where multiple vertices collapsed to the same cell)
        if v0 != v1 && v1 != v2 && v0 != v2 {
            new_faces.push([v0, v1, v2]);
        }
    }

    println!("Downsampled: {} -> {} vertices, {} -> {} triangles", 
        vertices.len(), new_vertices.len(), 
        faces.len(), new_faces.len());

    (new_vertices, new_faces)
}