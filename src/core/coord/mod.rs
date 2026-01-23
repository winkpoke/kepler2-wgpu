mod base;
pub use base::*;

#[cfg(test)]
mod tests {
    use super::*;
    use glam::{Mat4, Vec3};

    #[test]
    fn test_coordinate_system() {
        let base = Base {
            label: "test".to_string(),
            matrix: Mat4::IDENTITY,
        };
        assert_eq!(base.label, "test");
        let matrix = base.matrix;
        // matrix.col(col)[row]
        // Identity matrix: diagonals are 1.0
        assert_eq!(matrix.col(0)[0], 1.0);
        assert_eq!(matrix.col(1)[1], 1.0);
        assert_eq!(matrix.col(2)[2], 1.0);
        assert_eq!(matrix.col(3)[3], 1.0);
    }

    #[test]
    fn test_base_basic() {
        let m = [
            1., 0.5, 0., 0., 0., 1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1.,
        ];
        // Mat4::from_cols_array takes column-major array.
        // The original test used Matrix4x4::from_array which was from_rows.
        // So we need to be careful about the input data.
        // Original: Matrix4x4::from_array(m) -> from_rows(m).
        // So m was row-major.
        // m = [1, 0.5, 0, 0, ...] -> row 0 is [1, 0.5, 0, 0].
        // glam::Mat4::from_cols_array accepts column-major.
        // So we should use Mat4::from_cols_array(&m).transpose() to match "from_rows".

        let matrix = Mat4::from_cols_array(&m).transpose();
        println!("{:?}", matrix);

        // apply point
        let p = Vec3::new(3., 2., 1.);
        let res = matrix.transform_point3(p);
        println!("{:?}", res);

        let base0 = Base {
            label: "world coordinate".to_string(),
            matrix: Mat4::IDENTITY,
        };

        let base1 = Base {
            label: "system coordinate".to_string(),
            matrix: matrix,
        };
        let transform_matrix = base0.to_base(&base1);
        println!("{:?}", transform_matrix);
    }

    #[test]
    fn test_base_nontrivial() {
        // Original used Matrix4x4::from_rows
        let m0_rows = [
            -0.51469487,
            1.16777869,
            0.11198701,
            -0.44676615,
            -1.79107111,
            -1.18206274,
            -0.18222625,
            -1.25953278,
            1.72667095,
            1.85407961,
            2.36366226,
            1.58998366,
            0.0,
            0.0,
            0.0,
            1.0,
        ];
        let matrix0 = Mat4::from_cols_array(&m0_rows).transpose();

        let m1_rows = [
            -0.53832315,
            1.36244315,
            -0.11961783,
            2.41102403,
            1.17852419,
            -0.84371312,
            -1.13160416,
            -1.61392419,
            0.00636648,
            -0.7648334,
            -0.19224463,
            -0.09854762,
            0.0,
            0.0,
            0.0,
            1.0,
        ];
        let matrix1 = Mat4::from_cols_array(&m1_rows).transpose();

        println!("{:?}", matrix0);
        let p = Vec3::new(3., 2., 1.);
        let res = matrix1.transform_point3(p);
        println!("{:?}", res);

        let base0 = Base {
            label: "world coordinate".to_string(),
            matrix: matrix0,
        };
        let base1 = Base {
            label: "system coordinate".to_string(),
            matrix: matrix1,
        };
        let transform_matrix = base0.to_base(&base1);
        println!("{:?}", transform_matrix);
    }
}
