use crate::{coord::{Base, Matrix4x4, Vector3}, dicom::{DicomRepo, CTImage}, CTVolume};
use std::ops::{DivAssign, SubAssign};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

pub struct GeometryBuilder<'a> {
    repo: Option<&'a DicomRepo>,
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
    pub fn build_plane_from_points(vol: &CTVolume, x: Vector3<f32>, y: Vector3<f32>, translate: Vector3<f32>) -> Base<f32> {
        let (nx, ny, nz) = ( 
                vol.dimensions.0 as f32,
                vol.dimensions.1 as f32,
                vol.dimensions.2 as f32,
            );
        let (sp_x, sp_y, sp_z)= vol.voxel_spacing;

        let scale = f32::max(nx * sp_x, ny * sp_y);
        let scale_z = nz * sp_z;
        println!("scale :{:?}",scale);

        let z = x.cross(y);
        println!("z :{:?}",z);

        let matrix = Matrix4x4::from_array([
                        x.data[0]*scale,   y.data[0] *scale,  -z.data[0] *scale,   translate.data[0],
                        x.data[1]*scale,   y.data[1] *scale,  z.data[1] *scale,    translate.data[1],
                        x.data[2]*scale,   y.data[2] *scale,  z.data[2] *scale_z,  translate.data[2],
                        0.0,   0.0,    0.0,    1.0,]);

        Base { matrix, label: "Plane".into() }
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
    /// A `Base<f32>` struct representing the transformation from normalized UV space to
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
    /// ```
    // / let uv_base = build_uv_base(&ct_volume);
    // / let world_pos = uv_base.matrix.transform_point((u, v, w, 1.0));
    /// ```
    pub fn build_uv_base(vol: &CTVolume) -> Base<f32> {       
        let nx = vol.dimensions.0 as f32 - 1.0;
        let ny = vol.dimensions.1 as f32 - 1.0;
        let nz = vol.dimensions.2 as f32 - 1.0;

        let scaling_matrix = Matrix4x4::from_array(
            [nx,  0.0, 0.0, 0.0,
             0.0,  ny, 0.0, 0.0,
             0.0, 0.0,  nz, 0.0,
             0.0, 0.0, 0.0, 1.0]);

        let base_uv_matrix = vol.base.matrix.multiply(&scaling_matrix);

        let base_uv = Base::<f32> {
            label: "CT Volume: UV".to_string(),
            matrix: base_uv_matrix,
        };
        // println!("UV Base:\n{:?}", base_uv);
        return base_uv;

    }

    // pub fn build_transverse_base(vol: &CTVolume) -> Base<f32> {        
    //     let (nx, ny, nz) = (
    //         vol.dimensions.0 as f32,
    //         vol.dimensions.1 as f32,
    //         vol.dimensions.2 as f32,
    //     );

    //     let space = vol.voxel_spacing;

    //     let [ox, oy, oz, _] = vol.base.matrix.get_column(3);

    //     let d = f32::max(nx * space.0, ny * space.1);
    //     let dz = space.2 * nz;

    //     let matrix_screen = Matrix4x4::<f32>::from_array([
    //           d,  0.0,  0.0, ox,
    //         0.0,    d,  0.0, oy,
    //         0.0,  0.0,   dz, oz,
    //         0.0,  0.0,  0.0, 1.0
    //     ]);
    //     let base_screen = Base::<f32> {
    //         label: "CT Volume: screen".to_string(),
    //         matrix: matrix_screen,
    //     };
    //     base_screen
    // }
    pub fn build_transverse_base(vol: &CTVolume) -> Base<f32> {
        let [ox, oy, oz, _] = vol.base.matrix.get_column(3); 
        let p0 = Vector3::new([ox, oy, oz]);
        let x = Vector3::new([1.0, 0.0, 0.0]);
        let y = Vector3::new([0.0, 1.0, 0.0]);
        Self::build_plane_from_points(vol, x, y,p0)
    }

    // pub fn build_coronal_base(vol: &CTVolume) -> Base<f32> {
    //     let (nx, ny, nz) = (
    //         vol.dimensions.0 as f32,
    //         vol.dimensions.1 as f32,
    //         vol.dimensions.2 as f32,
    //     );

    //     let space = vol.voxel_spacing;
    //     let [ox, oy, oz, _] = vol.base.matrix.get_column(3);
    //     let d = f32::max(nx * space.0, ny * space.1);

    //     let matrix_screen = Matrix4x4::<f32>::from_array([
    //         d,    0.0,  0.0, ox,
    //         0.0,  0.0,  d,   oy + ny * space.1 / 2.0,
    //         0.0,  -d,   0.0, oz + nz * space.2 / 2.0 + d / 2.0,
    //         0.0,  0.0,  0.0, 1.0
    //     ]);
    //     let base_screen = Base::<f32> {
    //         label: "CT Volume: screen".to_string(),
    //         matrix: matrix_screen,
    //     };
    //     base_screen
    // }
    pub fn build_coronal_base(vol: &CTVolume) -> Base<f32> {
        let (nx, ny, nz) = (
            vol.dimensions.0 as f32,
            vol.dimensions.1 as f32,
            vol.dimensions.2 as f32,
        );

        let space = vol.voxel_spacing;
        let [ox, oy, oz, _] = vol.base.matrix.get_column(3);
        let d = f32::max(nx * space.0, ny * space.1);

        let p0 = Vector3::new([ox,oy + ny * space.1 / 2.0,  oz + nz * space.2 / 2.0 + d / 2.0]);
        let x = Vector3::new([1.0, 0.0, 0.0]);
        let y = Vector3::new([0.0, 0.0, -1.0]);
        Self::build_plane_from_points(vol, x, y,p0)
    }

    // pub fn build_sagittal_base(vol: &CTVolume) -> Base<f32> {   
    //     let (nx, ny, nz) = (
    //         vol.dimensions.0 as f32,
    //         vol.dimensions.1 as f32,
    //         vol.dimensions.2 as f32,
    //     );

    //     let space = vol.voxel_spacing;
    //     let [ox, oy, oz, _] = vol.base.matrix.get_column(3);
    //     let d = f32::max(nx * space.0, ny * space.1);

    //     let matrix_screen = Matrix4x4::<f32>::from_array([
    //         0.0, 0.0,   d, ox + nx * space.0 / 2.0,
    //           d, 0.0, 0.0, oy,
    //         0.0,  -d, 0.0, oz + nz * space.2 / 2.0 + d / 2.0,
    //         0.0, 0.0, 0.0, 1.0
    //     ]);
    //     let base_screen = Base::<f32> {
    //         label: "CT Volume: screen".to_string(),
    //         matrix: matrix_screen,
    //     };
    //     base_screen
    // }
    pub fn build_sagittal_base(vol: &CTVolume) -> Base<f32> {
        let (nx, ny, nz) = (
            vol.dimensions.0 as f32,
            vol.dimensions.1 as f32,
            vol.dimensions.2 as f32,
        );

        let space = vol.voxel_spacing;
        let [ox, oy, oz, _] = vol.base.matrix.get_column(3);
        let d = f32::max(nx * space.0, ny * space.1);

        let p0 = Vector3::new([ox + nx * space.0 / 2.0,oy,  oz + nz * space.2 / 2.0 + d / 2.0]);
        let y = Vector3::new([0.0, 0.0, -1.0]);
        let x = Vector3::new([0.0, 1.0, 0.0]);
        Self::build_plane_from_points(vol, x, y,p0)
    }

    // pub fn build_oblique_base(vol: &CTVolume) -> Base<f32> { 
    //     let (nx, ny, nz) = (
    //         vol.dimensions.0 as f32,
    //         vol.dimensions.1 as f32,
    //         vol.dimensions.2 as f32,
    //     );

    //     let space = vol.voxel_spacing;
    //     let [ox, oy, oz, _] = vol.base.matrix.get_column(3);
    //     let d = f32::max(nx * space.0, ny * space.1);
    //     let m_screen = [0.0,  0.0,    d/2.0, (ox + nx*space.0)/2.0 - d/2.0,
    //                     d,  0.0,    0.0, oy,
    //                     0.0,  -d,   0.0, oz + nz * space.2 / 2.0 + d / 2.0,
    //                     0.0,  0.0,  0.0, 1.0];
    //     let rotation = [ 0.9330,  0.2500, -0.2588, 0.0,
    //                     -0.1853,  0.9504,  0.2500, 0.0,     
    //                      0.3085, -0.1853,  0.9330, 0.0,
    //                         0.0,     0.0,     0.0, 1.0,]; 
    //     let matrix_screen = Matrix4x4::<f32>::from_array(m_screen);
    //     let matrix_rot = Matrix4x4::<f32>::from_array(rotation);
    //     let matrix_screen = matrix_screen * matrix_rot;
    //     let base_screen = Base::<f32> {
    //         label: "CT Volume: screen".to_string(),
    //         matrix: matrix_screen,
    //     };
    //     base_screen
    // }

    pub fn build_oblique_base(vol: &CTVolume) -> Base<f32> {
        let (nx, ny, nz) = (
            vol.dimensions.0 as f32,
            vol.dimensions.1 as f32,
            vol.dimensions.2 as f32,
        );

        let space = vol.voxel_spacing;
        let [ox, oy, oz, _] = vol.base.matrix.get_column(3);
        let d = f32::max(nx * space.0, ny * space.1);

        let p0 = Vector3::new([(ox + nx*space.0)/2.0 - d/2.0,oy ,  oz + nz * space.2 / 2.0 + d / 2.0]);
        let x = Vector3::new([0.3085, 0.9330, 0.1853]);
        let y = Vector3::new([-0.1853, 0.2500, -0.9504]);
        Self::build_plane_from_points(vol, x, y,p0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_uv_base() {
        let base0 = Base::<f32> {
            label: "world coordinate".to_string(),
            matrix: Matrix4x4::<f32>::eye(),
        };

        let volume_1 = crate::ct_volume::CTVolume{
            dimensions: (512, 512, 100),
            voxel_spacing: (0.5, 0.5, 1.0),
            voxel_data: vec![-1024; 512 * 512 * 100],
            base: base0,
        };

        let result = GeometryBuilder::build_uv_base(&volume_1);
        println!("Base:\n{:?}", result);
    }

    #[test]
    fn test_transverse_base() {
        let m = [0., 0., 0., -507.812, 
                0., 0., 0.,  -507.8125, 
                0., 0., 0.,  -923.5, 
                0., 0., 0., 1.];
        let base0 = Base::<f32> {
            label: "world coordinate".to_string(),
            matrix: Matrix4x4::<f32>::from_array(m),
        };

        let volume_1 = crate::ct_volume::CTVolume{
            dimensions: (512, 512, 100),
            voxel_spacing: (1.0, 1.0, 1.0),
            voxel_data: vec![-1024; 512 * 512 * 100],
            base: base0,
        };

        let result = GeometryBuilder::build_transverse_base(&volume_1);
        println!("transverse Base:\n{:?}", result);
        assert_eq!(result.matrix.data[0][0], 512.0);
        assert_eq!(result.matrix.data[1][1], 512.0);
        assert_eq!(result.matrix.data[2][2], 100.0);
    }

    #[test]
    fn test_coronal_base() {
        let m = [
            0., 0., 0., 10., 0., 0., 0., 5., 0., 0., 0., 3., 0., 0., 0., 1.,
        ];
        let base0 = Base::<f32> {
            label: "world coordinate".to_string(),
            matrix: Matrix4x4::<f32>::from_array(m),
        };

        let volume_1 = crate::ct_volume::CTVolume{
            dimensions: (512, 512, 100),
            voxel_spacing: (0.5, 0.5, 1.0),
            voxel_data: vec![-1024; 512 * 512 * 100],
            base: base0,
        };

        let result = GeometryBuilder::build_coronal_base(&volume_1);
        println!("coronal Base:\n{:?}", result);
        assert_eq!(result.matrix.data[2][1], -256.0);
        assert_eq!(result.matrix.data[1][3], (5.0+512.0*0.5/2.0));
    }

    #[test]
    fn test_sagittal_base() {
        let m = [
            0., 0., 0., 10., 0., 0., 0., 5., 0., 0., 0., 3., 0., 0., 0., 1.,
        ];
        let base0 = Base::<f32> {
            label: "world coordinate".to_string(),
            matrix: Matrix4x4::<f32>::from_array(m),
        };

        let volume_1 = crate::ct_volume::CTVolume{
            dimensions: (512, 512, 100),
            voxel_spacing: (0.5, 0.5, 1.0),
            voxel_data: vec![-1024; 512 * 512 * 100],
            base: base0,
        };

        let result = GeometryBuilder::build_sagittal_base(&volume_1);
        println!("sagittal Base:\n{:?}", result);
    }

    #[test]
    fn test_oblique_base() {
        let base0 = Base::<f32> {
            label: "world coordinate".to_string(),
            matrix: Matrix4x4::<f32>::eye(),
        };

        let volume_1 = crate::ct_volume::CTVolume{
            dimensions: (512, 512, 100),
            voxel_spacing: (0.5, 0.5, 1.0),
            voxel_data: vec![-1024; 512 * 512 * 100],
            base: base0,
        };

        let result = GeometryBuilder::build_oblique_base(&volume_1);
        println!("oblique Base:\n{:?}", result);
    }
}
