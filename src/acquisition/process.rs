use ndarray::Array2;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

const WIDTH: usize = 1024;
const HEIGHT: usize = 1024;

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