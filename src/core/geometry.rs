use crate::{core::coord::Base, data::{dicom::{DicomRepo, CTImage}, ct_volume::CTVolume}};
use glam::{Mat4, Vec3};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

pub struct GeometryBuilder<'a> {
    #[allow(dead_code)]
    repo: Option<&'a DicomRepo>,
    #[allow(dead_code)]
    sorted_image_series: Option<Vec<&'a CTImage>>, 
}

// #[cfg(not(target_arch = "wasm32"))]
impl <'a> GeometryBuilder<'a> {
    pub fn new() -> Self {
        Self {
            repo: None,
            sorted_image_series: None,
        }
    }

    pub fn dicom_repo(self, repo: &'a DicomRepo) -> Self {
        Self {
            repo: Some(repo),
            ..self
        }
    }

    /// Constructs a base transformation that maps normalized voxel coordinates (UV space)
    /// in the range [0.0, 1.0]³ to physical (world/patient) space using the CT volume's
    /// base matrix and dimensions.
    ///
    /// This is useful for transforming UV coordinates (e.g., from shaders, 3D rendering,
    /// or normalized input) into actual spatial positions in the DICOM coordinate system.
    ///
    /// # Arguments
    /// * `vol` - Reference to a `CTVolume` containing the dimensions, voxel spacing,
    ///           and the transformation matrix mapping voxel indices to world space.
    ///
    /// # Returns
    /// A `Base` struct representing the transformation from normalized UV space to
    /// world/patient coordinates.
    ///
    /// # Implementation Notes
    /// * The volume dimensions are reduced by one to convert counts to index ranges,
    ///   since voxel indices go from `0` to `N-1`.
    /// * A scaling matrix maps [0,1] UV coordinates to actual voxel index space.
    /// * The final transformation matrix is computed by multiplying the volume's
    ///   base matrix with the UV scaling matrix:
    ///   `world_matrix = vol.base.matrix * scaling_matrix`.
    ///
    /// # Example
    /// ```rust,ignore
    /// // This example requires a constructed CTVolume and is ignored in doctests.
    /// let uv_base = build_uv_base(&ct_volume);
    /// let world_pos = uv_base.matrix.transform_point3(Vec3::new(u, v, w));
    /// ```
    pub fn build_uv_base(vol: &CTVolume) -> Base {       
        let nx = vol.dimensions.0 as f32 - 1.0;
        let ny = vol.dimensions.1 as f32 - 1.0;
        let nz = vol.dimensions.2 as f32 - 1.0;

        let scaling_matrix = Mat4::from_scale(Vec3::new(nx, ny, nz));
        
        // vol.base.matrix is already Mat4 (since Base)
        let vol_matrix = vol.base.matrix;

        // Matrix multiplication in glam: result = A * B
        let base_uv_matrix_glam = vol_matrix * scaling_matrix;

        let base_uv = Base {
            label: "CT Volume: UV".to_string(),
            matrix: base_uv_matrix_glam,
        };
        // println!("UV Base:\n{:?}", base_uv);
        return base_uv;

    }

    pub fn build_transverse_base(vol: &CTVolume) -> Base {        
        let (nx, ny, nz) = (
            vol.dimensions.0 as f32,
            vol.dimensions.1 as f32,
            vol.dimensions.2 as f32,
        );

        let space = vol.voxel_spacing;
        let col3 = vol.base.matrix.col(3);
        let ox = col3.x;
        let oy = col3.y;
        let oz = col3.z;
        
        // Use per-axis physical extents for accuracy
        let (d_x, d_y, d_z) = (nx * space.0, ny * space.1, nz * space.2);
        
        // Isotropic 3D scaling - use average of in-plane and slice extents
        let d = (d_x + d_y + d_z) / 3.0;

        let matrix_screen_glam = Mat4::from_cols_array(&[
            // Screen X → world X (LR)
              d,  0.0,  0.0, ox + d_x / 2.0 - d / 2.0,
            // Screen Y → world Y (AP) - no inversion needed for transverse
            0.0,    d,  0.0, oy + d_y / 2.0 - d / 2.0,
            // Screen Z (slice) → world Z (SI)
            0.0,  0.0,    d, oz + d_z / 2.0,
            // Homogeneous row
            0.0,  0.0,  0.0, 1.0
        ]).transpose();

        let base_screen = Base {
            label: "CT Volume: screen".to_string(),
            matrix: matrix_screen_glam,
        };
        base_screen
    }

    pub fn build_coronal_base(vol: &CTVolume) -> Base {
        let (nx, ny, nz) = (
            vol.dimensions.0 as f32,
            vol.dimensions.1 as f32,
            vol.dimensions.2 as f32,
        );

        let space = vol.voxel_spacing;
        let col3 = vol.base.matrix.col(3);
        let ox = col3.x;
        let oy = col3.y;
        let oz = col3.z;
        // let d = f32::max(nx * space.0, ny * space.1);
        let (d_x, d_y, d_z) = (nx * space.0, ny * space.1, nz * space.2);

        // Isotropic 3D scaling - use average of in-plane and slice extents
        let d = (d_x + d_y + d_z) / 3.0;

        let matrix_screen_glam = Mat4::from_cols_array(&[
            // Screen X → world X (LR)
              d,  0.0,  0.0, ox + d_x / 2.0 - d / 2.0,
            // Screen Z (slice) → world Y (AP)
            0.0,  0.0,    d, oy + d_y / 2.0,
            // Screen Y → world Z (SI), inverted for screen Y-down
            0.0,   -d,  0.0, oz + d_z / 2.0 + d / 2.0,
            // Homogeneous row
            0.0,  0.0,  0.0, 1.0
        ]).transpose();

        let base_screen = Base {
            label: "CT Volume: screen".to_string(),
            matrix: matrix_screen_glam,
        };
        base_screen
    }

    pub fn build_sagittal_base(vol: &CTVolume) -> Base {   
        let (nx, ny, nz) = (
            vol.dimensions.0 as f32,
            vol.dimensions.1 as f32,
            vol.dimensions.2 as f32,
        );

        let space = vol.voxel_spacing;
        let col3 = vol.base.matrix.col(3);
        let ox = col3.x;
        let oy = col3.y;
        let oz = col3.z;
        // Use per-axis physical extents (mm)
        let (d_x, d_y, d_z) = (nx * space.0, ny * space.1, nz * space.2);

        // Isotropic 3D scaling - use average of in-plane and slice extents
        let d = (d_x + d_y + d_z) / 3.0;

        // Screen X → world Y (AP)
        // Screen Y → world Z (SI), inverted for screen Y-down
        // Screen Z (slice) → world X (LR)
        let matrix_screen_glam = Mat4::from_cols_array(&[
            // world X row
            0.0, 0.0,   d, ox + d_x / 2.0,
            // world Y row
              d, 0.0, 0.0, oy + d_y / 2.0 - d / 2.0,
            // world Z row
            0.0,  -d, 0.0, oz + d_z / 2.0 + d / 2.0,
            // homogeneous row
            0.0, 0.0, 0.0, 1.0
        ]).transpose();

        let base_screen = Base {
            label: "CT Volume: screen".to_string(),
            matrix: matrix_screen_glam,
        };
        base_screen
    }

    pub fn build_oblique_base(vol: &CTVolume) -> Base { 
        let (nx, ny, nz) = (
            vol.dimensions.0 as f32,
            vol.dimensions.1 as f32,
            vol.dimensions.2 as f32,
        );

        let space = vol.voxel_spacing;
        let col3 = vol.base.matrix.col(3);
        let ox = col3.x;
        let oy = col3.y;
        let oz = col3.z;
        let d = f32::max(nx * space.0, ny * space.1);
        let m_screen = [0.0,  0.0,    d/2.0, (ox + nx*space.0)/2.0 - d/2.0,
                        d,  0.0,    0.0, oy,
                        0.0,  -d,   0.0, oz + nz * space.2 / 2.0 + d / 2.0,
                        0.0,  0.0,  0.0, 1.0];
        let rotation = [ 0.9330,  0.2500, -0.2588, 0.0,
                        -0.1853,  0.9504,  0.2500, 0.0,     
                         0.3085, -0.1853,  0.9330, 0.0,
                            0.0,     0.0,     0.0, 1.0,]; 
        
        let matrix_screen_glam = Mat4::from_cols_array(&m_screen).transpose();
        let matrix_rot_glam = Mat4::from_cols_array(&rotation).transpose();

        // Original was: matrix_rot * matrix_screen
        let final_matrix = matrix_rot_glam * matrix_screen_glam;

        let base_screen = Base {
            label: "CT Volume: screen".to_string(),
            matrix: final_matrix,
        };
        base_screen
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_uv_base() {
        let base0 = Base {
            label: "world coordinate".to_string(),
            matrix: Mat4::IDENTITY,
        };

        let volume_1 = crate::ct_volume::CTVolume{
            dimensions: (512, 512, 100),
            voxel_spacing: (0.5, 0.5, 1.0),
            voxel_data: vec![-1024; 512 * 512 * 100],
            base: base0,
        };

        let result = GeometryBuilder::build_uv_base(&volume_1);
        assert!(result.label == "CT Volume: UV");
        // Access via col(i)[j] for Mat4
        assert_eq!(result.matrix.col(0)[0], 511.0);
        assert_eq!(result.matrix.col(1)[1], 511.0);
        assert_eq!(result.matrix.col(2)[2], 99.0);
    }

    #[test]
    fn test_transverse_base() {
        let m = [0., 0., 0., -507.812, 
                0., 0., 0.,  -507.8125, 
                0., 0., 0.,  -923.5, 
                0., 0., 0., 1.];
        let base0 = Base {
            label: "world coordinate".to_string(),
            matrix: Mat4::from_cols_array(&m).transpose(), // from_cols_array reads as col-major, but m is row-major
        };

        let volume_1 = crate::ct_volume::CTVolume{
            dimensions: (512, 512, 100),
            voxel_spacing: (1.0, 1.0, 1.0),
            voxel_data: vec![-1024; 512 * 512 * 100],
            base: base0,
        };

        let result = GeometryBuilder::build_transverse_base(&volume_1);
        assert!(result.label == "CT Volume: screen");
        let (nx, ny, nz) = (volume_1.dimensions.0 as f32, volume_1.dimensions.1 as f32, volume_1.dimensions.2 as f32);
        let (dx, dy, dz) = (nx * volume_1.voxel_spacing.0, ny * volume_1.voxel_spacing.1, nz * volume_1.voxel_spacing.2);
        let d = (dx + dy + dz) / 3.0;
        
        assert!((result.matrix.col(0)[0] - d).abs() < 1e-6);
        // oy + dy/2 - d/2
        let expected_y = -507.8125 + dy / 2.0 - d / 2.0;
        assert!((result.matrix.col(3)[1] - expected_y).abs() < 1e-6);
    }

    #[test]
    fn test_coronal_base() {
        let m = [
            0., 0., 0., 10., 0., 0., 0., 5., 0., 0., 0., 3., 0., 0., 0., 1.,
        ];
        let base0 = Base {
            label: "world coordinate".to_string(),
            matrix: Mat4::from_cols_array(&m).transpose(),
        };

        let volume_1 = crate::ct_volume::CTVolume{
            dimensions: (512, 512, 100),
            voxel_spacing: (0.5, 0.5, 1.0),
            voxel_data: vec![-1024; 512 * 512 * 100],
            base: base0,
        };

        let result = GeometryBuilder::build_coronal_base(&volume_1);
        let (nx, ny, nz) = (volume_1.dimensions.0 as f32, volume_1.dimensions.1 as f32, volume_1.dimensions.2 as f32);
        let (dx, dy, dz) = (nx * volume_1.voxel_spacing.0, ny * volume_1.voxel_spacing.1, nz * volume_1.voxel_spacing.2);
        let d = (dx + dy + dz) / 3.0;
        assert!((result.matrix.col(1)[2] + d).abs() < 1e-6);
        assert_eq!(result.matrix.col(3)[1], (5.0 + dy / 2.0));
    }

    #[test]
    fn test_sagittal_base() {
        let m = [
            0., 0., 0., 10., 0., 0., 0., 5., 0., 0., 0., 3., 0., 0., 0., 1.,
        ];
        let base0 = Base {
            label: "world coordinate".to_string(),
            matrix: Mat4::from_cols_array(&m).transpose(),
        };

        let volume_1 = crate::ct_volume::CTVolume{
            dimensions: (512, 512, 100),
            voxel_spacing: (0.5, 0.5, 1.0),
            voxel_data: vec![-1024; 512 * 512 * 100],
            base: base0,
        };

        let result = GeometryBuilder::build_sagittal_base(&volume_1);
        assert!(result.label == "CT Volume: screen");
        let (nx, ny, nz) = (volume_1.dimensions.0 as f32, volume_1.dimensions.1 as f32, volume_1.dimensions.2 as f32);
        let (dx, dy, dz) = (nx * volume_1.voxel_spacing.0, ny * volume_1.voxel_spacing.1, nz * volume_1.voxel_spacing.2);
        let d = (dx + dy + dz) / 3.0;
        assert!((result.matrix.col(0)[1] - d).abs() < 1e-6);
        let expected_z = 3.0 + dz / 2.0 + d / 2.0;
        assert!((result.matrix.col(3)[2] - expected_z).abs() < 1e-6);
    }

    #[test]
    fn test_oblique_base() {
        let base0 = Base {
            label: "world coordinate".to_string(),
            matrix: Mat4::IDENTITY,
        };

        let volume_1 = crate::ct_volume::CTVolume{
            dimensions: (512, 512, 100),
            voxel_spacing: (0.5, 0.5, 1.0),
            voxel_data: vec![-1024; 512 * 512 * 100],
            base: base0,
        };

        let result = GeometryBuilder::build_oblique_base(&volume_1);
        // data[row][col] -> columns[col][row]
        // Mat4 stores col-major. col(i)[j] is j-th element of i-th column.
        assert!((result.matrix.col(2)[0] - 119.424).abs() < 1e-3);
        assert!((result.matrix.col(2)[1] - -23.7184).abs() < 1e-3);
        assert!((result.matrix.col(2)[2] - 39.488).abs() < 1e-3);
        assert!((result.matrix.col(2)[3] - 0.0).abs() < 1e-3);
    }
}
