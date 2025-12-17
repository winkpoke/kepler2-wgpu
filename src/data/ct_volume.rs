#![allow(dead_code)]

use anyhow::Result;
use std::fmt;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

use crate::core::coord::Base;
use crate::core::coord::Matrix4x4;


// Define the CTVolume struct to hold 3D data
#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
#[derive(Clone)]
pub struct CTVolume {
    pub(crate) dimensions: (usize, usize, usize), // (rows, columns, number of slices)
    pub(crate) voxel_spacing: (f32, f32, f32), // (spacing_x, spacing_y, spacing_z)
    // pub(crate) voxel_data: Vec<Vec<i16>>, // 3D voxel data flattened into slices
    pub(crate) voxel_data: Vec<i16>, // 3D voxel data 
    pub(crate) base: Base<f32>,
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
    /// Function-level comment: Creates a new CTVolume containing only the data within the specified ROI (Region of Interest).
    /// The start coordinates and dimensions are in voxel space.
    pub fn crop(&self, start: (usize, usize, usize), size: (usize, usize, usize)) -> Result<CTVolume> {
        let (x0, y0, z0) = start;
        let (sx, sy, sz) = size;
        let (w, h, d) = self.dimensions;

        if x0 + sx > w || y0 + sy > h || z0 + sz > d {
            return Err(anyhow::anyhow!("Crop region out of bounds: start={:?}, size={:?}, dim={:?}", start, size, self.dimensions));
        }

        let mut new_voxel_data = Vec::with_capacity(sx * sy * sz);
        let stride_z = w * h;
        let stride_y = w;

        for z in 0..sz {
            let src_z = z0 + z;
            for y in 0..sy {
                let src_y = y0 + y;
                let start_idx = src_z * stride_z + src_y * stride_y + x0;
                let end_idx = start_idx + sx;
                new_voxel_data.extend_from_slice(&self.voxel_data[start_idx..end_idx]);
            }
        }

        // Calculate new origin in world space
        let origin_voxel = [x0 as f32, y0 as f32, z0 as f32];
        let new_origin_world = self.base.matrix.multiply_point3(origin_voxel);

        // Construct new base matrix
        // The orientation and scaling remain the same, only the translation (last column) changes.
        let mut new_matrix_data = self.base.matrix.data;
        new_matrix_data[0][3] = new_origin_world[0];
        new_matrix_data[1][3] = new_origin_world[1];
        new_matrix_data[2][3] = new_origin_world[2];

        let new_base = Base {
            label: format!("{}_cropped", self.base.label),
            matrix: Matrix4x4 { data: new_matrix_data },
        };

        Ok(CTVolume {
            dimensions: (sx, sy, sz),
            voxel_spacing: self.voxel_spacing,
            voxel_data: new_voxel_data,
            base: new_base,
        })
    }

    /// Crops the volume using a bounding box defined in world coordinates (mm).
    /// The resulting volume will be aligned with the original voxel grid,
    /// encompassing the specified world region.
    pub fn crop_by_world_bounds(&self, world_min: [f32; 3], world_max: [f32; 3]) -> Result<CTVolume> {
        // Define the 8 corners of the world space bounding box
        let corners = [
            [world_min[0], world_min[1], world_min[2]],
            [world_min[0], world_min[1], world_max[2]],
            [world_min[0], world_max[1], world_min[2]],
            [world_min[0], world_max[1], world_max[2]],
            [world_max[0], world_min[1], world_min[2]],
            [world_max[0], world_min[1], world_max[2]],
            [world_max[0], world_max[1], world_min[2]],
            [world_max[0], world_max[1], world_max[2]],
        ];

        let mut min_v = [f32::MAX; 3];
        let mut max_v = [f32::MIN; 3];

        let inv_matrix = self.base.matrix.inv().ok_or_else(|| anyhow::anyhow!("Singular matrix, cannot invert transform"))?;

        // Transform all corners to voxel space to find the extent in voxel grid
        for p in corners {
            let v_pos = inv_matrix.multiply_point3(p);
            for i in 0..3 {
                if v_pos[i] < min_v[i] { min_v[i] = v_pos[i]; }
                if v_pos[i] > max_v[i] { max_v[i] = v_pos[i]; }
            }
        }

        // Clamp to volume dimensions and convert to indices
        let start_x = (min_v[0].floor() as isize).max(0) as usize;
        let start_y = (min_v[1].floor() as isize).max(0) as usize;
        let start_z = (min_v[2].floor() as isize).max(0) as usize;

        let end_x = (max_v[0].ceil() as isize).min(self.dimensions.0 as isize) as usize;
        let end_y = (max_v[1].ceil() as isize).min(self.dimensions.1 as isize) as usize;
        let end_z = (max_v[2].ceil() as isize).min(self.dimensions.2 as isize) as usize;
        
        if start_x >= end_x || start_y >= end_y || start_z >= end_z {
             return Err(anyhow::anyhow!("Crop region is empty or outside volume bounds"));
        }

        let size = (end_x - start_x, end_y - start_y, end_z - start_z);
        let start = (start_x, start_y, start_z);
        self.crop(start, size)
    }
}

pub trait CTVolumeGenerator {
    fn generate_ct_volume(&self, image_series_id: &str) -> Result<CTVolume>;
}

pub struct Geometry {
    volumes: Vec<CTVolume>,
    base: crate::core::coord::Matrix4x4<f32>,
}
