#![allow(dead_code)]

use anyhow::Result;
use std::fmt;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

use crate::core::coord::Base;
use glam::{Mat4, Vec2, Vec3};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;
#[cfg(target_arch = "wasm32")]
use js_sys::Array;

// Define the CTVolume struct to hold 3D data
#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
#[derive(Clone)]
pub struct CTVolume {
    pub(crate) dimensions: (usize, usize, usize), // (rows, columns, number of slices)
    pub(crate) voxel_spacing: (f32, f32, f32), // (spacing_x, spacing_y, spacing_z)
    // pub(crate) voxel_data: Vec<Vec<i16>>, // 3D voxel data flattened into slices
    pub(crate) voxel_data: Vec<i16>, // 3D voxel data 
    pub(crate) base: Base,
}

impl fmt::Debug for CTVolume {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CTVolume")
            .field("dimensions", &self.dimensions)
            .field("voxel_spacing", &self.voxel_spacing)
            .field("voxel_data", &format!("{} slices", self.voxel_data.len()))
            .field("base", &format!("\n{:?}", &self.base))
            .finish()
    }
}

impl CTVolume {
    pub fn new(
        dimensions: (usize, usize, usize),
        voxel_spacing: (f32, f32, f32),
        voxel_data: Vec<i16>,
        base: Base,
    ) -> Self {
        Self {
            dimensions,
            voxel_spacing,
            voxel_data,
            base,
        }
    }

    pub fn dimensions(&self) -> (usize, usize, usize) {
        self.dimensions
    }

    pub fn voxel_spacing(&self) -> (f32, f32, f32) {
        self.voxel_spacing
    }

    pub fn voxel_data(&self) -> &[i16] {
        &self.voxel_data
    }

    pub fn base(&self) -> &Base {
        &self.base
    }
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
#[cfg(target_arch = "wasm32")]
impl CTVolume {
    #[wasm_bindgen(js_name = export_mha)]
    pub fn export_mha(&self) -> js_sys::Uint8Array {
        let mut mha_data = Vec::new();
        
        // Extract offset (translation) from base matrix
        let offset_x = self.base.matrix.w_axis.x;
        let offset_y = self.base.matrix.w_axis.y;
        let offset_z = self.base.matrix.w_axis.z;
        
        // Calculate direction cosines by removing scaling
        let col0 = self.base.matrix.x_axis.truncate();
        let col1 = self.base.matrix.y_axis.truncate();
        let col2 = self.base.matrix.z_axis.truncate();
        
        let dir0 = col0.normalize_or_zero();
        let dir1 = col1.normalize_or_zero();
        let dir2 = col2.normalize_or_zero();

        // MHA TransformMatrix is row-major (or ITK standard is column vectors flattened?)
        // Standard ITK direction matrix is typically written as:
        // dir0.x dir1.x dir2.x dir0.y dir1.y dir2.y dir0.z dir1.z dir2.z
        let header = format!(
            "ObjectType = Image\n\
            NDims = 3\n\
            BinaryData = True\n\
            BinaryDataByteOrderMSB = False\n\
            CompressedData = False\n\
            TransformMatrix = {} {} {} {} {} {} {} {} {}\n\
            Offset = {} {} {}\n\
            CenterOfRotation = 0 0 0\n\
            AnatomicalOrientation = RAI\n\
            ElementSpacing = {} {} {}\n\
            DimSize = {} {} {}\n\
            ElementType = MET_SHORT\n\
            ElementDataFile = LOCAL\n",
            dir0.x, dir1.x, dir2.x,
            dir0.y, dir1.y, dir2.y,
            dir0.z, dir1.z, dir2.z,
            offset_x, offset_y, offset_z,
            self.voxel_spacing.0, self.voxel_spacing.1, self.voxel_spacing.2,
            self.dimensions.0, self.dimensions.1, self.dimensions.2
        );

        mha_data.extend_from_slice(header.as_bytes());
        
        // Append raw voxel data (i16 little endian)
        // Reserve exact capacity
        mha_data.reserve_exact(self.voxel_data.len() * 2);
        for v in &self.voxel_data {
            mha_data.extend_from_slice(&v.to_le_bytes());
        }
        
        js_sys::Uint8Array::from(mha_data.as_slice())
    }

    #[wasm_bindgen(js_name = export_mha_with_effects)]
    pub fn export_mha_with_effects(
        &self,
        apply_noise: bool,
        noise_intensity: f32,
        js_matrix: JsValue,
    ) -> js_sys::Uint8Array {
        let mut mha_data = Vec::new();

        // 解析前端传来的矩阵 (Float32Array)
        let float32_array = js_sys::Float32Array::new(&js_matrix);
        let mut m = vec![0.0f32; 16];
        if float32_array.length() == 16 {
            float32_array.copy_to(&mut m);
        } else {
            // 如果不是 16 个元素，给一个单位阵
            m[0] = 1.0; m[5] = 1.0; m[10] = 1.0; m[15] = 1.0;
        }

        let col0 = Vec3::new(m[0], m[1], m[2]);
        let col1 = Vec3::new(m[4], m[5], m[6]);
        let col2 = Vec3::new(m[8], m[9], m[10]);
        let offset = Vec3::new(m[12], m[13], m[14]);

        let dir0 = if col0.length() > 0.0 { col0.normalize() } else { Vec3::X };
        let dir1 = if col1.length() > 0.0 { col1.normalize() } else { Vec3::Y };
        let dir2 = if col2.length() > 0.0 { col2.normalize() } else { Vec3::Z };

        let header = format!(
            "ObjectType = Image\n\
            NDims = 3\n\
            BinaryData = True\n\
            BinaryDataByteOrderMSB = False\n\
            CompressedData = False\n\
            TransformMatrix = {} {} {} {} {} {} {} {} {}\n\
            Offset = {} {} {}\n\
            CenterOfRotation = 0 0 0\n\
            AnatomicalOrientation = RAI\n\
            ElementSpacing = {} {} {}\n\
            DimSize = {} {} {}\n\
            ElementType = MET_SHORT\n\
            ElementDataFile = LOCAL\n",
            dir0.x, dir1.x, dir2.x,
            dir0.y, dir1.y, dir2.y,
            dir0.z, dir1.z, dir2.z,
            offset.x, offset.y, offset.z,
            self.voxel_spacing.0, self.voxel_spacing.1, self.voxel_spacing.2,
            self.dimensions.0, self.dimensions.1, self.dimensions.2
        );

        mha_data.extend_from_slice(header.as_bytes());
        mha_data.reserve_exact(self.voxel_data.len() * 2);

        // 原有高斯噪声逻辑
        if apply_noise {
            let dim_x = self.dimensions.0 as f32;
            let dim_y = self.dimensions.1 as f32;
            let dim_z = self.dimensions.2 as f32;

            for z in 0..self.dimensions.2 {
                for y in 0..self.dimensions.1 {
                    for x in 0..self.dimensions.0 {
                        let idx = x + y * self.dimensions.0 + z * self.dimensions.0 * self.dimensions.1;
                        let v = self.voxel_data[idx] as f32;
                        let uv = Vec2::new(x as f32 / dim_x * 512.0, y as f32 / dim_y * 512.0);
                        let noise = Self::gaussian_noise(uv) * noise_intensity;
                        let factor = ((v + 800.0) / 1100.0).clamp(0.0, 1.0);
                        let noise_scaled = noise * (1.0 - factor);
                        let noisy_v = if v < 1000.0 { (v + noise_scaled).clamp(-32768.0, 32767.0) } else { v };
                        mha_data.extend_from_slice(&(noisy_v as i16).to_le_bytes());
                    }
                }
            }
        } else {
            for &v in &self.voxel_data {
                mha_data.extend_from_slice(&v.to_le_bytes());
            }
        }

        js_sys::Uint8Array::from(mha_data.as_slice())
    }

    fn fract(x: f32) -> f32 { x - x.floor() }

    fn rand(p: Vec2) -> f32 {
        Self::fract((p.x * 12.9898 + p.y * 78.233).sin() * 43758.5453)
    }

    // 近似高斯，但严格在 [-1,1]
    fn gaussian_noise(p: Vec2) -> f32 {
        let n1 = Self::rand(p);
        let n2 = Self::rand(p + Vec2::new(0.37, 0.17));
        let n3 = Self::rand(p + Vec2::new(1.17, 2.31));
        let n4 = Self::rand(p + Vec2::new(2.11, 0.73));
        (n1 + n2 + n3 + n4) / 2.0 - 1.0
    }
}


pub struct Geometry {
    volumes: Vec<CTVolume>,
    base: Mat4,
}

pub trait CTVolumeGenerator {
    fn generate_ct_volume(&self, image_series_id: &str) -> Result<CTVolume>;
}
