use crate::{
    core::coord::Base,
    data::{
        ct_volume::CTVolume,
        dicom::{CTImage, DicomRepo},
    },
};
use glam::{Mat4, Vec3};

pub struct GeometryBuilder<'a> {
    #[allow(dead_code)]
    repo: Option<&'a DicomRepo>,
    #[allow(dead_code)]
    sorted_image_series: Option<Vec<&'a CTImage>>,
}

// #[cfg(not(target_arch = "wasm32"))]
impl<'a> GeometryBuilder<'a> {
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
    ///   and the transformation matrix mapping voxel indices to world space.
    ///
    /// # Returns
    /// A `Base` struct representing the transformation from normalized UV space to
    /// world/patient coordinates.
    ///
    /// # Implementation Notes
    /// * The volume dimensions are used directly to ensure full coverage.
    /// * Texture centres (i+0.5)/N map to voxel-centre index i, including half-voxel boundaries.
    /// * The final transformation matrix is computed by multiplying the volume's
    ///   base matrix with the UV scaling matrix:
    ///   `world_matrix = vol.base.matrix * translate(-0.5) * scaling_matrix`.
    ///
    /// # Example
    /// ```rust,ignore
    /// // This example requires a constructed CTVolume and is ignored in doctests.
    /// let uv_base = build_uv_base(&ct_volume);
    /// let world_pos = uv_base.matrix.transform_point3(Vec3::new(u, v, w));
    /// ```
    pub fn build_uv_base(vol: &CTVolume) -> Base {
        let nx = vol.dimensions.0 as f32;
        let ny = vol.dimensions.1 as f32;
        let nz = vol.dimensions.2 as f32;

        let scaling_matrix = Mat4::from_scale(Vec3::new(nx, ny, nz));

        // vol.base.matrix is already Mat4 (since Base)
        let vol_matrix = vol.base.matrix;

        // Matrix multiplication in glam: result = A * B
        let base_uv_matrix_glam =
            vol_matrix * Mat4::from_translation(Vec3::splat(-0.5)) * scaling_matrix;

        let base_uv = Base {
            label: "CT Volume: UV".to_string(),
            matrix: base_uv_matrix_glam,
        };
        // println!("UV Base:\n{:?}", base_uv);
        base_uv
    }

    /// Centre of voxel centres, transformed by the complete source affine.
    pub fn volume_center(vol: &CTVolume) -> Vec3 {
        let (x, y, z) = vol.dimensions;
        vol.base.matrix.transform_point3(Vec3::new(
            x.saturating_sub(1) as f32 * 0.5,
            y.saturating_sub(1) as f32 * 0.5,
            z.saturating_sub(1) as f32 * 0.5,
        ))
    }

    /// Centred, uniformly normalized physical box for DVR and its UV-space overlays.
    /// Original world translation is removed only for camera framing; axis directions
    /// and relative physical lengths remain those of the source affine.
    pub fn normalized_texture_to_world(vol: &CTVolume) -> Mat4 {
        let uv = Self::build_uv_base(vol).matrix;
        let extent =
            uv.x_axis.truncate().abs() + uv.y_axis.truncate().abs() + uv.z_axis.truncate().abs();
        let scale = extent.max_element().max(f32::EPSILON).recip();
        Mat4::from_translation(Vec3::splat(0.5))
            * Mat4::from_scale(Vec3::splat(scale))
            * Mat4::from_translation(-Self::volume_center(vol))
            * uv
    }

    fn screen_base(vol: &CTVolume, right: Vec3, down: Vec3, normal: Vec3) -> Base {
        let (x, y, z) = vol.dimensions;
        let edges = [
            vol.base.matrix.x_axis.truncate() * x as f32,
            vol.base.matrix.y_axis.truncate() * y as f32,
            vol.base.matrix.z_axis.truncate() * z as f32,
        ];
        // Projection of all box edges fits rotations/reflections without assuming axis alignment.
        let extent = |axis: Vec3| edges.iter().map(|e| e.dot(axis).abs()).sum::<f32>();
        let d = extent(right)
            .max(extent(down))
            .max(extent(normal))
            .max(f32::EPSILON);
        let center = Self::volume_center(vol);
        Base {
            label: "CT Volume: screen".into(),
            matrix: Mat4::from_cols(
                (right * d).extend(0.0),
                (down * d).extend(0.0),
                (normal * d).extend(0.0),
                (center - (right + down) * (d * 0.5)).extend(1.0),
            ),
        }
    }

    pub fn build_transverse_base(vol: &CTVolume) -> Base {
        Self::screen_base(vol, Vec3::X, Vec3::Y, Vec3::Z)
    }

    pub fn build_coronal_base(vol: &CTVolume) -> Base {
        Self::screen_base(vol, Vec3::X, -Vec3::Z, Vec3::Y)
    }

    pub fn build_sagittal_base(vol: &CTVolume) -> Base {
        Self::screen_base(vol, Vec3::Y, -Vec3::Z, Vec3::X)
    }

    pub fn build_oblique_base(vol: &CTVolume) -> Base {
        let rotation = glam::Quat::from_euler(glam::EulerRot::XYZ, 0.25, -0.25, 0.25);
        Self::screen_base(
            vol,
            rotation * Vec3::X,
            rotation * Vec3::Y,
            rotation * Vec3::Z,
        )
    }
}

impl<'a> Default for GeometryBuilder<'a> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn screen_centres_and_texture_centres_share_the_full_affine() {
        let affine = Mat4::from_translation(Vec3::new(10.0, -20.0, 30.0))
            * Mat4::from_rotation_z(0.7)
            * Mat4::from_scale(Vec3::new(-0.5, 0.75, 2.0));
        let vol = CTVolume::new(
            (8, 6, 4),
            (0.5, 0.75, 2.0),
            vec![0; 192],
            Base {
                label: "fixture".into(),
                matrix: affine,
            },
        );
        let center = affine.transform_point3(Vec3::new(3.5, 2.5, 1.5));
        let uv = GeometryBuilder::build_uv_base(&vol).matrix;
        assert!(uv
            .transform_point3(Vec3::splat(0.5))
            .abs_diff_eq(center, 1e-5));
        for build in [
            GeometryBuilder::build_transverse_base,
            GeometryBuilder::build_coronal_base,
            GeometryBuilder::build_sagittal_base,
            GeometryBuilder::build_oblique_base,
        ] {
            let screen = build(&vol).matrix;
            assert!(screen
                .transform_point3(Vec3::new(0.5, 0.5, 0.0))
                .abs_diff_eq(center, 1e-5));
            assert!((uv.inverse() * screen)
                .transform_point3(Vec3::new(0.5, 0.5, 0.0))
                .abs_diff_eq(Vec3::splat(0.5), 1e-5));
        }
        let first = uv.transform_point3(Vec3::new(0.5 / 8.0, 0.5 / 6.0, 0.5 / 4.0));
        assert!(first.abs_diff_eq(affine.transform_point3(Vec3::ZERO), 1e-5));
    }
}
