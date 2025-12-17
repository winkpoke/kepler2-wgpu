mod dicom_reader;
mod marching_tetrahedra;
mod mesh_processing;
mod obj_writer;
mod image_processing;

use anyhow::Result;
use dicom_reader::DicomReader;
use marching_tetrahedra::MarchingTetrahedra;
use mesh_processing::{laplacian_smooth, merge_vertices, downsample_mesh};
use ndarray::{s, ArrayView3};
use obj_writer::{ObjWriter, NamedMesh};
use std::time::Instant;

struct Tissue {
    name: String,
    min: i16,
    max: i16,
    color: [f32; 3],
}

fn main() -> Result<()> {
    // Settings
    let dicom_folder = r"E:\dataset\medical_data\SE00001";
    let output_file = "output_mesh_mt.obj";
    
    // Smoothing settings
    let smooth_iterations = 5;
    let smooth_lambda = 0.5f32;
    let downsample_grid_size = 2.0;

    // Gaussian Filter settings
    let enable_gaussian = true;
    let gaussian_sigma = 0.5f32;

    // Chunk settings
    let chunk_size = 500;

    let tissues = vec![
        Tissue {
            name: "Bone_Cortical".to_string(),
            min: 50,
            max: 400,
            color: [0.95, 0.90, 0.85],
        },
    ];
    
    // ROI definition (Cube vertices in Physical Coordinates mm)
    // Modify these values to customize the Region of Interest
    let roi_vertices: Vec<[f64; 3]> = vec![
        [-190.9629364013672, -91.0, -1257.0],    // Min Point
        [200.0, 149.0, -932.0] // Max Point
    ];

    println!("Starting Marching Tetrahedra conversion (Chunked)...");
    let start_total = Instant::now();

    // 1. Scan DICOM Series
    println!("\n=== Scanning DICOM Series ===");
    let start_scan = Instant::now();
    let series_info = DicomReader::scan_dicom_series(dicom_folder)?;
    println!("Scan time: {:.2?}", start_scan.elapsed());

    let total_depth = series_info.files.len();
    println!("Total Slices: {}", total_depth);

    // Initialize OBJ Writer
    let mut obj_writer = ObjWriter::new(output_file)?;

    // 2. Process Chunks
    // We start processing at 0. Step by chunk_size.
    // Overlap is used for reading and Gaussian filter context.
    let overlap = 4; // Sufficient for Gaussian (sigma 0.5 -> kernel radius ~2) + Derivatives
    // Compute ROI AABB
    let (roi_min, roi_max) = {
        let mut min = [f64::INFINITY; 3];
        let mut max = [f64::NEG_INFINITY; 3];
        for v in &roi_vertices {
            for i in 0..3 {
                if v[i] < min[i] { min[i] = v[i]; }
                if v[i] > max[i] { max[i] = v[i]; }
            }
        }
        (min, max)
    };
    println!("ROI: Min {:?}, Max {:?}", roi_min, roi_max);

    // Calculate ROI Slice Range (Z)
    // Z index = (z_phys - origin_z) / spacing_z
    let origin_z = series_info.origin.2;
    // Use average slice thickness from series_info or verify? series_info.slice_thickness
    // Assuming uniform spacing for index calculation.
    let z_min_idx = ((roi_min[2] - origin_z) / series_info.slice_thickness).floor() as isize;
    let z_max_idx = ((roi_max[2] - origin_z) / series_info.slice_thickness).ceil() as isize;
    
    let process_z_start = z_min_idx.max(0) as usize;
    let process_z_end = z_max_idx.min(total_depth as isize) as usize;
    
    println!("Processing Z Range: {}..{}", process_z_start, process_z_end);

    let mut chunk_index = 0;
   
    // Start loop from process_z_start, go up to process_z_end
    for z_start in (process_z_start..process_z_end).step_by(chunk_size) {
        let z_end = (z_start + chunk_size).min(total_depth);

        if z_end <= z_start { break; }

        println!("\n--- Processing Chunk {} (Logical Slices {} to {}) ---", chunk_index, z_start, z_end);
        
        // 1. Calculate Read Range (with Overlap)
        let read_start = z_start.saturating_sub(overlap);
        let read_end = (z_end + overlap).min(total_depth);
        
        // If read range is too small (e.g. at end of file), handle gracefully
        if read_end - read_start < 2 { break; }

        // Load Chunk
        let start_read = Instant::now();
        let (mut volume, spacing) = DicomReader::read_dicom_chunk(&series_info, read_start, read_end, 256)?;
        println!("Read Chunk ({}..{}) Time: {:.2?}", read_start, read_end, start_read.elapsed());

        // Apply Gaussian Filter (on full read context)
        if enable_gaussian {
             let start_gauss = Instant::now();
             volume = image_processing::apply_gaussian_filter(volume, gaussian_sigma);
             println!("Gaussian Filter Time: {:.2?}", start_gauss.elapsed());
        }

        // Calculate ROI indices for X and Y using current spacing
        // spacing is (z, y, x)
        let (_, spacing_y, spacing_x) = spacing;
        let origin_x = series_info.origin.0;
        let origin_y = series_info.origin.1;

        // X Range
        let x_min_idx = ((roi_min[0] - origin_x) / spacing_x).floor() as isize;
        let x_max_idx = ((roi_max[0] - origin_x) / spacing_x).ceil() as isize;
        let x_dim = volume.len_of(ndarray::Axis(2)) as isize;
        let crop_x_start = x_min_idx.max(0) as usize;
        let crop_x_end = x_max_idx.min(x_dim) as usize;

        // Y Range
        let y_min_idx = ((roi_min[1] - origin_y) / spacing_y).floor() as isize;
        let y_max_idx = ((roi_max[1] - origin_y) / spacing_y).ceil() as isize;
        let y_dim = volume.len_of(ndarray::Axis(1)) as isize;
        let crop_y_start = y_min_idx.max(0) as usize;
        let crop_y_end = y_max_idx.min(y_dim) as usize;
        
        // Check if ROI overlaps with volume in X/Y
        if crop_x_end <= crop_x_start || crop_y_end <= crop_y_start {
            println!("Chunk outside ROI in X/Y plane, skipping.");
            chunk_index += 1;
            continue;
        }

        let cropped_volume = volume.slice(s![.., crop_y_start..crop_y_end, crop_x_start..crop_x_end]);
        println!("Cropped ROI (Local): X({}..{}), Y({}..{})", crop_x_start, crop_x_end, crop_y_start, crop_y_end);


        // Calculate Valid Range for Extraction
        // volume covers read_start..read_end
        // We want to extract for cells in z_start..z_end
        // Local index = Global index - read_start
        let local_start = z_start - read_start;
        let local_end = z_end - read_start;
        // z_range for extraction: local_start..local_end
        let extract_range = local_start..local_end;

        // Determine Pinned Z coordinates (Global)
        let mut pinned_zs = Vec::new();
        let spacing_z = spacing.0 as f32; // Extract Z spacing as f32 for comparison
        
        // Pin start boundary if not the very first slice of volume
        if z_start > 0 {
            pinned_zs.push(z_start as f32 * spacing_z);
        }
        // Pin end boundary if not the very last slice
        if z_end < total_depth {
            pinned_zs.push(z_end as f32 * spacing_z);
        }
        
        if !pinned_zs.is_empty() {
            println!("Pinning boundaries at Z: {:?}", pinned_zs);
        }

        // Process Tissues
        for tissue in &tissues {
            println!("Processing {}...", tissue.name);
            let start_mc = Instant::now();
            
            let mt = MarchingTetrahedra::new(tissue.min, tissue.max);
            let mt = MarchingTetrahedra::new(tissue.min, tissue.max);
            
            // Offset for coordinate calculation: [x_start, y_start, z_start]
            // Arrays are [z, y, x].
            // read_start is global Z.
            // crop_x_start is local X equivalent to global X since we just access slices (unless we implement tiled reading later)
            // wait, if we resized, crop_x_start is index in resized volume.
            // But get_pos uses spacing to go back to physical.
            // So if spacing is correct, (x + offset) * spacing = physical.
            // Yes.
            let offset = [crop_x_start, crop_y_start, read_start];
            
            // (vertices, faces) = mt.extract_surface(&volume, spacing, offset, ...);
            let (vertices, faces) = mt.extract_surface(&cropped_volume, spacing, offset, extract_range.clone());
            
            println!("Generated {} vertices, {} triangles", vertices.len(), faces.len());

            if !vertices.is_empty() {
                let (merged_vertices, merged_faces) = merge_vertices(&vertices, &faces);

                // Apply Smoothing with Pinning
                let smoothed_vertices = laplacian_smooth(&merged_vertices, &merged_faces, smooth_iterations, smooth_lambda, &pinned_zs);
                
                let (downsampled_vertices, downsampled_faces) = downsample_mesh(&smoothed_vertices, &merged_faces, downsample_grid_size, &pinned_zs);
                println!("Processing Time: {:.2?}", start_mc.elapsed());

                obj_writer.write_chunk(&NamedMesh {
                    name: tissue.name.clone(),
                    color: tissue.color,
                    vertices: downsampled_vertices,
                    faces: downsampled_faces,
                })?;
            }
        }
        chunk_index += 1;
    }

    // 3. Finish
    obj_writer.flush()?;
    println!("\nTotal time: {:.2?}", start_total.elapsed());

    Ok(())
}