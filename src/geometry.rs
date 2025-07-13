use crate::{coord::{Base, Matrix4x4}, dicom::DicomRepo, CTImage, CTVolume};
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

    pub fn scale<T: DivAssign + Copy>(mat: &mut Matrix4x4<T>, scale: T) {
        for i in 0..3 {
            for j in 0..3 {
                mat.data[i][j] /= scale;
            }
        }
    }

    pub fn translate<T: SubAssign + Copy>(mat: &mut Matrix4x4<T>, translate: [T; 3]) {
        for i in 0..3 {
            mat.data[i][3] -= translate[i];
        }
    }

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
        println!("UV Base:\n{:?}", base_uv);
        return base_uv;

    }

    pub fn build_transverse_base(vol: &CTVolume) -> Base<f32> {        
        let nx = vol.dimensions.0 as f32 - 1.0;
        let ny = vol.dimensions.1 as f32 - 1.0;
        let nz = vol.dimensions.2 as f32 - 1.0;

        let space = vol.voxel_spacing;

        let [ox, oy, oz, _] = vol.base.matrix.get_column(3);

        let d = f32::max(nx * space.0, ny * space.1);
        let dz = space.2 * nz;

        let matrix_screen = Matrix4x4::<f32>::from_array([
              d,  0.0,  0.0, ox,
            0.0,    d,  0.0, oy,
            0.0,  0.0,   dz, oz,
            0.0,  0.0,  0.0, 1.0
        ]);
        let base_screen = Base::<f32> {
            label: "CT Volume: screen".to_string(),
            matrix: matrix_screen,
        };
        base_screen
    }

    pub fn build_sagittal_base(vol: &CTVolume) -> Base<f32> {
        let nx = vol.dimensions.0 as f32 - 1.0;
        let ny = vol.dimensions.1 as f32 - 1.0;
        let nz = vol.dimensions.2 as f32 - 1.0;

        let space = vol.voxel_spacing;

        let [ox, oy, oz, _] = vol.base.matrix.get_column(3);

        let d = f32::max(nx * space.0, ny * space.1);
        let dz = space.2 * nz;

        let s = 1.5;
        let t = 200.0;
        let matrix_screen = Matrix4x4::<f32>::from_array([
            d/s,    0.0,  0.0, ox - t,
            0.0,  0.0,  d / 4.0/s,  (oy + ny * space.1) / 2.0 - d / 2.8,
            0.0,  -d/s,   0.0, oz + nz * space.2 / 2.0 + d/2.0 - t,
            0.0,  0.0,  0.0, 1.0
        ]);
        let base_screen = Base::<f32> {
            label: "CT Volume: screen".to_string(),
            matrix: matrix_screen,
        };
        base_screen
    }

    pub fn build_coronal_base(vol: &CTVolume) -> Base<f32> {   
        let nx = vol.dimensions.0 as f32 - 1.0;
        let ny = vol.dimensions.1 as f32 - 1.0;
        let nz = vol.dimensions.2 as f32 - 1.0;

        let space = vol.voxel_spacing;

        let [ox, oy, oz, _] = vol.base.matrix.get_column(3);

        let d = f32::max(nx * space.0, ny * space.1);
        let dz = space.2 * nz;

        let matrix_screen = Matrix4x4::<f32>::from_array([
            0.0,  0.0,    d/2.0, (ox + nx*space.0)/2.0 - d/2.0,
            d,  0.0,    0.0, oy,
            0.0,  -d,   0.0, oz + nz * space.2 / 2.0 + d / 2.0,
            0.0,  0.0,  0.0, 1.0
        ]);
        let base_screen = Base::<f32> {
            label: "CT Volume: screen".to_string(),
            matrix: matrix_screen,
        };
        println!("d = {}", d);
        base_screen
    }

    pub fn build_oblique_base(vol: &CTVolume) -> Base<f32> { 
        let nx = vol.dimensions.0 as f32 - 1.0;
        let ny = vol.dimensions.1 as f32 - 1.0;
        let nz = vol.dimensions.2 as f32 - 1.0;

        let space = vol.voxel_spacing;

        let [ox, oy, oz, _] = vol.base.matrix.get_column(3);

        let d = f32::max(nx * space.0, ny * space.1);
        let dz = space.2 * nz;

        let m_screen = [0.0,  0.0,    d/2.0, (ox + nx*space.0)/2.0 - d/2.0,
                        d,  0.0,    0.0, oy,
                        0.0,  -d,   0.0, oz + nz * space.2 / 2.0 + d / 2.0,
                        0.0,  0.0,  0.0, 1.0];
        let rotation = [ 0.9330,  0.2500, -0.2588, 0.0,
                        -0.1853,  0.9504,  0.2500, 0.0,     
                         0.3085, -0.1853,  0.9330, 0.0,
                            0.0,     0.0,     0.0, 1.0,]; 
        let matrix_screen = Matrix4x4::<f32>::from_array(m_screen);
        let matrix_rot = Matrix4x4::<f32>::from_array(rotation);
        let matrix_screen = matrix_screen * matrix_rot;
        let base_screen = Base::<f32> {
            label: "CT Volume: screen".to_string(),
            matrix: matrix_screen,
        };
        base_screen
    }
}

