#![allow(dead_code)]

use anyhow::Result;
use std::fmt;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

use crate::core::coord::Base;
use glam::Mat4;

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
}


pub struct Geometry {
    volumes: Vec<CTVolume>,
    base: Mat4,
}

pub trait CTVolumeGenerator {
    fn generate_ct_volume(&self, image_series_id: &str) -> Result<CTVolume>;
}
