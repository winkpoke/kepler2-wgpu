use std::collections::HashMap;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
// Function-level comment: Import DICOM, ndarray, and stats extensions from the correct crates
// Use dicom-object and dicom-core crates declared in Cargo.toml; ndarray and ndarray-stats provide array ops
use dicom_object::open_file;
use dicom_core::Tag;
use ndarray::Array3;
use ndarray_stats::QuantileExt;

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
        volume: &Array3<f32>,
        iso_value: f32,
        spacing: (f32, f32, f32),
        downsample: Option<usize>,
        vertex_precision: usize,
    ) -> (Vec<[f32; 3]>, Vec<[u32; 3]>) {
        println!("原始体数据大小: {:?}", volume.shape());

        let mut volume = volume.clone();
        let mut final_spacing = spacing;

        if let Some(ds) = downsample {
            if ds > 1 {
                println!("降采样因子: {}", ds);
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
                // 修复1: 使用更新后的spacing
                final_spacing = (spacing.0 * ds as f32, spacing.1 * ds as f32, spacing.2 * ds as f32);
                println!("降采样后大小: {:?}", volume.shape());
                println!("更新后的体素间距: {:?}", final_spacing);
            }
        }

        let shape = volume.shape();
        let depth = shape[0];
        let height = shape[1];
        let width = shape[2];

        println!("筛选活跃区域...");
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
        println!("活跃体素数: {}", active_indices.len());

        if active_indices.is_empty() {
            println!("ERR: 未找到包含等值面的体素");
            return (vec![], vec![]);
        }

        let mut vertex_dict: HashMap<(i32, i32, i32), usize> = HashMap::new();
        let mut vertices: Vec<[f32; 3]> = vec![];
        let mut triangles: Vec<[u32; 3]> = vec![];

        let total = active_indices.len();
        for (idx, &(z, y, x)) in active_indices.iter().enumerate() {
            if idx % 10000 == 0 {
                println!("处理进度: {}/{} ({:.1}%)", idx, total, 100.0 * idx as f32 / total as f32);
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
                [x as f32 * final_spacing.2, y as f32 * final_spacing.1, z as f32 * final_spacing.0],
                [(x + 1) as f32 * final_spacing.2, y as f32 * final_spacing.1, z as f32 * final_spacing.0],
                [(x + 1) as f32 * final_spacing.2, (y + 1) as f32 * final_spacing.1, z as f32 * final_spacing.0],
                [x as f32 * final_spacing.2, (y + 1) as f32 * final_spacing.1, z as f32 * final_spacing.0],
                [x as f32 * final_spacing.2, y as f32 * final_spacing.1, (z + 1) as f32 * final_spacing.0],
                [(x + 1) as f32 * final_spacing.2, y as f32 * final_spacing.1, (z + 1) as f32 * final_spacing.0],
                [(x + 1) as f32 * final_spacing.2, (y + 1) as f32 * final_spacing.1, (z + 1) as f32 * final_spacing.0],
                [x as f32 * final_spacing.2, (y + 1) as f32 * final_spacing.1, (z + 1) as f32 * final_spacing.0],
            ];

            for &tet_indices in self.cube_to_tetrahedra.iter() {
                // 修复2: 正确提取四面体的顶点位置和值
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

        println!("\n生成完成: {} 个顶点, {} 个三角形", vertices.len(), triangles.len());

        if vertices.is_empty() {
            return (vec![], vec![]);
        }

        (vertices, triangles)
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
                        0.5  // 修复3: 使用0.5而不是0.0
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

fn load_dicom_volume(dicom_dir: &str, max_slices: Option<usize>) -> (Array3<f32>, (f32, f32, f32)) {
    println!("从 {} 加载DICOM文件...", dicom_dir);
    let path = Path::new(dicom_dir);
    let mut dicom_files: Vec<PathBuf> = std::fs::read_dir(path)
        .unwrap()
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().map_or(false, |ext| ext == "dcm"))
        .collect();
    dicom_files.sort();

    if dicom_files.is_empty() {
        panic!("未在 {} 中找到DICOM文件", dicom_dir);
    }

    if let Some(max_s) = max_slices {
        dicom_files.truncate(max_s);
    }

    println!("找到 {} 个DICOM文件", dicom_files.len());

    let mut slices = dicom_files
        .iter()
        .map(|f| open_file(f).unwrap())
        .collect::<Vec<_>>();

    slices.sort_by(|a, b| {
        let pos_a = a.element(Tag(0x0020, 0x0032))
            .ok()
            .and_then(|e| e.to_multi_float64().ok())
            .unwrap_or_default();
        let pos_b = b.element(Tag(0x0020, 0x0032))
            .ok()
            .and_then(|e| e.to_multi_float64().ok())
            .unwrap_or_default();
        
        if pos_a.is_empty() || pos_b.is_empty() {
            std::cmp::Ordering::Equal
        } else {
            pos_a[2].partial_cmp(&pos_b[2]).unwrap_or(std::cmp::Ordering::Equal)
        }
    });

    let pixel_spacing = slices[0].element(Tag(0x0028, 0x0030)).unwrap().to_multi_float64().unwrap();
    let slice_thickness = slices[0]
        .element(Tag(0x0018, 0x0050))
        .map_or(1.0, |e| e.to_float64().unwrap_or(1.0));
    let spacing = (slice_thickness as f32, pixel_spacing[1] as f32, pixel_spacing[0] as f32);

    println!("体素间距: {:?}", spacing);

    let height = match slices[0].element(Tag(0x0028, 0x0010))
        .ok()
        .and_then(|e| e.to_int::<u32>().ok()) {
        Some(h) => h as usize,
        None => {
            eprintln!("错误: 无法读取图像高度");
            return (Array3::zeros((0, 0, 0)), (1.0, 1.0, 1.0));
        }
    };
    let width = match slices[0].element(Tag(0x0028, 0x0011))
        .ok()
        .and_then(|e| e.to_int::<u32>().ok()) {
        Some(w) => w as usize,
        None => {
            eprintln!("错误: 无法读取图像宽度");
            return (Array3::zeros((0, 0, 0)), (1.0, 1.0, 1.0));
        }
    };
    let depth = slices.len();

    let mut volume = Array3::zeros((depth, height, width));

    for (z, slice) in slices.iter().enumerate() {
        let pixel_data = match slice.element(Tag(0x7fe0, 0x0010))
            .ok()
            .and_then(|e| e.to_bytes().ok()) {
            Some(data) => data,
            None => {
                eprintln!("警告: 无法读取第 {} 帧的像素数据，跳过", z);
                continue;
            }
        };
        
        let mut slice_data = Array3::zeros((1, height, width));
        for (i, chunk) in pixel_data.chunks(2).enumerate() {
            if i < height * width {
                let value = u16::from_le_bytes([chunk[0], chunk.get(1).copied().unwrap_or(0)]) as f32;
                let row = i / width;
                let col = i % width;
                if row < height && col < width {
                    slice_data[[0, row, col]] = value;
                }
            }
        }
        volume.slice_mut(ndarray::s![z, .., ..]).assign(&slice_data.slice(ndarray::s![0, .., ..]));
    }

    println!("体数据形状: {:?}", volume.shape());
    // println!("数值范围: [{}, {}]", volume.min().unwrap(), volume.max().unwrap());

    (volume, spacing)
}

fn save_obj(filename: &str, vertices: &[[f32; 3]], triangles: &[[u32; 3]]) {
    println!("保存到 {}...", filename);
    println!("顶点数: {}", vertices.len());
    println!("三角形数: {}", triangles.len());

    let mut file = BufWriter::new(File::create(filename).unwrap());

    for v in vertices.iter() {
        writeln!(file, "v {:.6} {:.6} {:.6}", v[0], v[1], v[2]).unwrap();
    }

    for t in triangles.iter() {
        writeln!(file, "f {} {} {}", t[0] + 1, t[1] + 1, t[2] + 1).unwrap();
    }

    let file_size_mb = std::fs::metadata(filename).unwrap().len() as f32 / (1024.0 * 1024.0);
    println!("保存完成: 文件大小 {:.2} MB", file_size_mb);
}

// fn main() {
//     println!("\n{}", "=".repeat(50));

//     let dicom_dir = r"C:\share\imrt";

//     let (volume, spacing) = load_dicom_volume(dicom_dir, None);

//     let mt = MarchingTetrahedra::new();
//     // 修复4: 降低vertex_precision以提高顶点合并率
//     let (vertices, triangles) = mt.extract_isosurface(
//         &volume,
//         100.0,
//         spacing,
//         Some(3),
//         0,  // 从1改为0，提高顶点合并容差
//     );

//     if !vertices.is_empty() {
//         save_obj("dicom_surface_smooth.obj", &vertices, &triangles);
//     }
// }