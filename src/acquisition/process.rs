use std::fs::{self, File};
use std::io::{BufWriter, Read, Write};
use std::path::{Path, PathBuf};
use ndarray::{Array2, Array3, Axis};
use byteorder::{LittleEndian, WriteBytesExt};
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

// ============================================================
// Configuration
// ============================================================
struct Config {
    first_proj_index: usize,
    last_proj_index: usize,
}

impl Config {
    fn proj_indices(&self) -> std::ops::Range<usize> {
        self.first_proj_index..self.last_proj_index
    }
}

const CONFIG: Config = Config {
    first_proj_index: 0,
    last_proj_index: 621,
};

const WIDTH: usize = 1024;
const HEIGHT: usize = 1024;
const N_FRAMES: usize = 50;

// Read raw u16 data from a file
fn read_raw_u16(path: &Path) -> Result<Array2<u16>, Box<dyn std::error::Error>> {
    let mut file = File::open(path)?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;
    if buffer.len() != WIDTH * HEIGHT * 2 {
        return Err(format!(
            "Unexpected file size for {}: got {} bytes, expected {}",
            path.display(), buffer.len(), WIDTH * HEIGHT * 2
        ).into());
    }
    let data: Vec<u16> = buffer.chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect();
    Ok(Array2::from_shape_vec((HEIGHT, WIDTH), data)?)
}

fn write_raw_f32(path: &Path, data: &Array3<f32>) -> Result<(), Box<dyn std::error::Error>> {
    let mut file = BufWriter::new(File::create(path)?);
    for &v in data.iter() {
        file.write_f32::<LittleEndian>(v)?;
    }
    file.flush()?;
    Ok(())
}

// ============================================================
// Save MHD + RAW (MetaImage)
// ============================================================
fn save_mhd(file_name: &Path, raw: &Array3<f32>) -> Result<(), Box<dyn std::error::Error>> {
    let folder = file_name.parent().unwrap_or(Path::new("."));
    fs::create_dir_all(folder)?;

    let basename = file_name
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("output");

    let raw_path = folder.join(format!("{}.raw", basename));

    println!(
        "Writing: {} ({:.1} MB)",
        raw_path.display(),
        (raw.len() * 4) as f64 / 1e6
    );

    write_raw_f32(&raw_path, raw)?;

    let num_of_slices = CONFIG.last_proj_index - CONFIG.first_proj_index;

    let mhd_content = format!(
        r#"ObjectType = Image
NDims = 3
BinaryData = True
BinaryDataByteOrderMSB = False
CompressedData = False
TransformMatrix = 1 0 0 0 1 0 0 0 1
Offset = -213.296 -213.296 0
CenterOfRotation = 0 0 0
AnatomicalOrientation = RAI
ElementSpacing = 0.417 0.417 1
ElementType = MET_FLOAT
DimSize = 1024 1024 {num_of_slices}
ElementDataFile = {basename}.raw
"#
    );

    fs::write(file_name, mhd_content)?;
    Ok(())
}

// Average multiple frames (dark / bright field)
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(js_name = compute_average_frames)]
pub fn average_frames(frames: &[i16], n_frames: usize) -> Vec<f32> {
    let total_pixels = WIDTH * HEIGHT;
    assert_eq!(frames.len(), total_pixels * n_frames);
    let mut acc = vec![0f64; total_pixels];
    for frame in frames.chunks_exact(total_pixels) {
        for (i, &v) in frame.iter().enumerate() {
            acc[i] += v as f64;
        }
    }
    acc.into_iter().map(|v| (v / n_frames as f64).round() as f32).collect()
}

pub fn load_average_raw_from_bytes(bytes: &[u8]) -> Result<Array2<f32>, Box<dyn std::error::Error>> {
    let expected_size = WIDTH * HEIGHT * 2;
    if bytes.len() != expected_size {
        return Err(format!(
            "Invalid raw size: {} bytes, expected {} bytes",
            bytes.len(),
            expected_size
        ).into());
    }

    let values: Vec<f32> = bytes.chunks_exact(2).map(|b| {
        u16::from_le_bytes([b[0], b[1]]) as f32
    }).collect();

    Ok(Array2::from_shape_vec((HEIGHT, WIDTH),values)?)
}

// Flat-field correction + negative log
pub fn process_raw(
    input: Vec<i16>,
    avg_dark: &Array2<f32>,
    avg_bright: &Array2<f32>,
    eps: f32,
) -> Result<Vec<i16>, Box<dyn std::error::Error>> {
    let mut processed = Array2::from_shape_vec(
        (HEIGHT, WIDTH),
        input.into_iter().map(|v| v as f32).collect(),
    )?;

    // Dark subtraction
    processed -= avg_dark;

    // Bright normalization: (I - D) / (B - D)
    let mut denom = avg_bright - avg_dark;
    denom.mapv_inplace(|v| v.max(eps));
    processed /= &denom;

    // Clamp to avoid log(0) / log(<0) and non-physical values > 1
    // 防止 log(0) / log(<0)，并把"比亮场还亮"的非物理样本裁掉
    processed.mapv_inplace(|v| v.max(eps).min(1.0));

    // Negative log → attenuation
    let q = processed.mapv(|v| -v.ln());
    let pixel: Vec<i16> = q.iter().map(|&v| (v * 1000.0).round() as i16).collect();
    Ok(pixel)
}