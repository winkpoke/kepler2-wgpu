//! Source-looking perspective geometry for beam's-eye review. All lengths are millimetres.
use crate::core::{KeplerError, KeplerResult};
use glam::{DMat4, DVec2, DVec3};

fn invalid(message: &str) -> KeplerError {
    KeplerError::Validation(format!("BEV: {message}"))
}

/// Validated orthonormal collimator basis; +Z points from isocenter to source.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BeamProjection {
    pub(crate) isocenter: DVec3,
    pub(crate) x: DVec3,
    pub(crate) y: DVec3,
    pub(crate) z: DVec3,
    pub(crate) sad: f64,
}

impl BeamProjection {
    /// Reject nonfinite/nonorthonormal geometry and nonpositive source-axis distance.
    pub fn new(isocenter: DVec3, x: DVec3, y: DVec3, z: DVec3, sad: f64) -> KeplerResult<Self> {
        let value = Self {
            isocenter,
            x,
            y,
            z,
            sad,
        };
        value.validate()?;
        Ok(value)
    }
    pub fn isocenter(&self) -> DVec3 {
        self.isocenter
    }
    pub fn source_position(&self) -> DVec3 {
        self.isocenter + self.z * self.sad
    }
    pub fn validate(&self) -> KeplerResult<()> {
        if !self.isocenter.is_finite()
            || !self.sad.is_finite()
            || self.sad <= 0.
            || [self.x, self.y, self.z]
                .iter()
                .any(|v| !v.is_finite() || (v.length_squared() - 1.).abs() > 1e-6)
            || self.x.dot(self.y).abs() > 1e-6
            || self.x.cross(self.y).dot(self.z) < 1. - 1e-6
        {
            return Err(invalid(
                "source and collimator basis must be finite and orthonormal",
            ));
        }
        Ok(())
    }
    /// Project to the isocenter plane. Points on/behind the source plane are absent.
    pub fn project(&self, patient: DVec3) -> Option<DVec2> {
        let q = patient - self.isocenter;
        let denominator = self.sad - q.dot(self.z);
        if !q.is_finite() || !denominator.is_finite() || denominator <= 1e-6 {
            return None;
        }
        let uv = DVec2::new(q.dot(self.x), q.dot(self.y)) * (self.sad / denominator);
        uv.is_finite().then_some(uv)
    }
    /// Patient LPS directions at screen left/right/top/bottom for X right/Y up.
    pub fn edge_directions(&self) -> [String; 4] {
        [-self.x, self.x, self.y, -self.y].map(|v| {
            let mut axes = [0usize, 1, 2];
            axes.sort_by(|&a, &b| v[b].abs().total_cmp(&v[a].abs()));
            axes.into_iter()
                .filter(|&axis| v[axis].abs() > 1e-4)
                .map(|axis| match (axis, v[axis] > 0.) {
                    (0, true) => 'L',
                    (0, false) => 'R',
                    (1, true) => 'P',
                    (1, false) => 'A',
                    (2, true) => 'S',
                    _ => 'I',
                })
                .collect()
        })
    }

    /// Conservative centered fit for supplied field vertices and optional patient box.
    pub fn fit_half_span(
        &self,
        volume: Option<&VolumeGeometry>,
        field: impl IntoIterator<Item = DVec2>,
    ) -> f64 {
        let mut half = 5_f64;
        for point in field {
            if point.is_finite() {
                half = half.max(point.abs().max_element());
            }
        }
        if let Some(volume) = volume {
            for point in volume.corners() {
                if let Some(p) = self.project(point) {
                    half = half.max(p.abs().max_element());
                }
            }
        }
        (half * 1.08).clamp(1., 10_000.)
    }
}

/// Physical voxel-center grid plus its existing normalized display-to-native permutation.
#[derive(Clone, Debug)]
pub struct VolumeGeometry {
    dimensions: [usize; 3],
    origin: DVec3,
    steps: [DVec3; 3],
    patient_to_texture: DMat4,
}

impl VolumeGeometry {
    /// `origin` is voxel (0,0,0)'s center in patient LPS; steps include signed spacing.
    /// `display_to_native` must map the normalized unit cube bijectively to itself.
    pub fn new(
        dimensions: [usize; 3],
        origin: DVec3,
        steps: [DVec3; 3],
        display_to_native: DMat4,
    ) -> KeplerResult<Self> {
        if dimensions.contains(&0)
            || dimensions
                .into_iter()
                .try_fold(1_usize, usize::checked_mul)
                .is_none()
            || !origin.is_finite()
            || steps.iter().any(|s| !s.is_finite() || s.length() <= 1e-6)
            || !display_to_native.is_finite()
            || (display_to_native.determinant().abs() - 1.).abs() > 1e-8
            || display_to_native.row(3) != glam::DVec4::W
        {
            return Err(invalid(
                "invalid physical voxel grid or native texture permutation",
            ));
        }
        for a in 0..3 {
            for b in a + 1..3 {
                if steps[a].normalize().dot(steps[b].normalize()).abs() > 1e-4 {
                    return Err(invalid("sheared voxel grids are unsupported"));
                }
            }
        }
        for i in 0..8 {
            let q = display_to_native.transform_point3(DVec3::new(
                f64::from(i & 1),
                f64::from((i >> 1) & 1),
                f64::from((i >> 2) & 1),
            ));
            if q.cmplt(DVec3::splat(-1e-8)).any() || q.cmpgt(DVec3::splat(1. + 1e-8)).any() {
                return Err(invalid(
                    "native texture transform does not preserve the unit cube",
                ));
            }
        }
        let voxel_to_patient = DMat4::from_cols(
            steps[0].extend(0.),
            steps[1].extend(0.),
            steps[2].extend(0.),
            origin.extend(1.),
        );
        let dims = DVec3::new(
            dimensions[0] as f64,
            dimensions[1] as f64,
            dimensions[2] as f64,
        );
        let mapping = display_to_native
            * DMat4::from_scale(dims.recip())
            * DMat4::from_translation(DVec3::splat(0.5))
            * voxel_to_patient.inverse();
        if !mapping.is_finite() {
            return Err(invalid("patient-to-texture mapping exceeds numeric range"));
        }
        Ok(Self {
            dimensions,
            origin,
            steps,
            patient_to_texture: mapping,
        })
    }
    pub fn patient_to_texture(&self) -> DMat4 {
        self.patient_to_texture
    }
    pub fn minimum_spacing_mm(&self) -> f64 {
        self.steps
            .iter()
            .map(|s| s.length())
            .fold(f64::INFINITY, f64::min)
    }
    pub fn corners(&self) -> [DVec3; 8] {
        std::array::from_fn(|i| {
            let p = std::array::from_fn::<_, 3, _>(|axis| {
                if (i >> axis) & 1 == 0 {
                    -0.5
                } else {
                    self.dimensions[axis] as f64 - 0.5
                }
            });
            self.origin + self.steps[0] * p[0] + self.steps[1] * p[1] + self.steps[2] * p[2]
        })
    }
}

/// Detector rectangle in collimator coordinates, ordered left/top to right/bottom.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DetectorRect {
    pub top_left: DVec2,
    pub extent: DVec2,
}

impl DetectorRect {
    /// Width and height are positive; screen rows move toward collimator -Y.
    pub fn new(top_left: DVec2, extent: DVec2) -> KeplerResult<Self> {
        if !top_left.is_finite() || !extent.is_finite() || extent.min_element() < 1e-6 {
            return Err(invalid("invalid detector extent"));
        }
        Ok(Self { top_left, extent })
    }
}

/// Original contour segments projected without reconstruction, filling, or silhouette inference.
#[derive(Clone, Debug)]
pub struct ProjectedRoi {
    pub index: usize,
    pub segments: Vec<[DVec2; 2]>,
}

/// Bounded projection. Exceeding the point limit is an error, never silent truncation.
pub fn project_contours(
    set: &super::structures::StructureSet,
    beam: &BeamProjection,
    selected: Option<usize>,
    point_limit: usize,
) -> KeplerResult<Vec<ProjectedRoi>> {
    beam.validate()?;
    let eligible = |i: usize| selected.is_none_or(|index| index == i);
    let count = set
        .rois
        .iter()
        .enumerate()
        .filter(|(i, _)| eligible(*i))
        .flat_map(|(_, roi)| roi.contours.iter())
        .try_fold(0usize, |sum, c| sum.checked_add(c.points().len()));
    if count.is_none_or(|count| count > point_limit) {
        return Err(invalid(
            "projected contours exceed the display point limit; select a smaller ROI",
        ));
    }
    let mut result = Vec::new();
    for (index, roi) in set.rois.iter().enumerate().filter(|(i, _)| eligible(*i)) {
        let mut segments = Vec::new();
        for contour in &roi.contours {
            let points = contour.points();
            let mut append = |a: DVec3, b: DVec3| {
                if let Some((a, b)) = beam.project(a).zip(beam.project(b)) {
                    segments.push([a, b]);
                }
            };
            for edge in points.windows(2) {
                append(edge[0], edge[1]);
            }
            if contour.kind().is_closed() {
                if let (Some(&first), Some(&last)) = (points.first(), points.last()) {
                    if first != last {
                        append(last, first);
                    }
                }
            }
        }
        result.push(ProjectedRoi { index, segments });
    }
    Ok(result)
}

/// Ideal blocked leaf rectangles, bounded for presentation; closed/crossed pairs block all.
pub fn leaf_rectangles(
    boundaries: &[f64],
    positions: &[f64],
    x_limits: [f64; 2],
) -> KeplerResult<Vec<[DVec2; 2]>> {
    let pairs = boundaries
        .len()
        .checked_sub(1)
        .ok_or_else(|| invalid("missing MLC boundaries"))?;
    if pairs == 0
        || pairs > 128
        || positions.len() != pairs * 2
        || !x_limits
            .iter()
            .chain(boundaries)
            .chain(positions)
            .all(|v| v.is_finite())
        || boundaries.windows(2).any(|p| p[0] >= p[1])
        || x_limits[0] >= x_limits[1]
    {
        return Err(invalid("invalid MLC layer geometry"));
    }
    let mut rects = Vec::with_capacity(pairs * 2);
    for i in 0..pairs {
        let left = positions[i].clamp(x_limits[0], x_limits[1]);
        let right = positions[i + pairs].clamp(x_limits[0], x_limits[1]);
        if left >= right {
            rects.push([
                DVec2::new(x_limits[0], boundaries[i]),
                DVec2::new(x_limits[1], boundaries[i + 1]),
            ]);
        } else {
            for [a, b] in [[x_limits[0], left], [right, x_limits[1]]] {
                if a < b {
                    rects.push([
                        DVec2::new(a, boundaries[i]),
                        DVec2::new(b, boundaries[i + 1]),
                    ]);
                }
            }
        }
    }
    Ok(rects)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn perspective_is_source_looking_and_rejects_source_plane() -> KeplerResult<()> {
        let b = BeamProjection::new(
            DVec3::new(10., 20., 30.),
            DVec3::X,
            DVec3::Y,
            DVec3::Z,
            1000.,
        )?;
        assert_eq!(
            b.project(b.isocenter + DVec3::new(20., -10., 500.)),
            Some(DVec2::new(40., -20.))
        );
        assert_eq!(
            b.project(b.isocenter + DVec3::new(20., -10., -1000.)),
            Some(DVec2::new(10., -5.))
        );
        assert!(b.project(b.isocenter + DVec3::Z * 1000.).is_none());
        assert!(BeamProjection::new(DVec3::ZERO, DVec3::X, DVec3::X, DVec3::Z, 1000.).is_err());
        Ok(())
    }
    #[test]
    fn anisotropic_and_reflected_native_mapping_keeps_half_voxel_centers() -> KeplerResult<()> {
        let native = DMat4::from_translation(DVec3::X) * DMat4::from_scale(DVec3::new(-1., 1., 1.));
        let volume = VolumeGeometry::new(
            [10, 20, 30],
            DVec3::new(100., 200., 300.),
            [DVec3::Y * 2., -DVec3::X * 3., DVec3::Z * 4.],
            native,
        )?;
        assert!(
            (volume
                .patient_to_texture()
                .transform_point3(DVec3::new(97., 202., 304.))
                - DVec3::new(0.85, 0.075, 0.05))
            .length()
                < 1e-12
        );
        Ok(())
    }
    #[test]
    fn crossed_leaf_pair_is_completely_blocked() -> KeplerResult<()> {
        let rects = leaf_rectangles(&[-5., 5.], &[3., -2.], [-10., 10.])?;
        assert_eq!(rects, vec![[DVec2::new(-10., -5.), DVec2::new(10., 5.)]]);
        Ok(())
    }

    #[test]
    fn projected_contours_preserve_original_edges_and_fail_on_point_limit() -> KeplerResult<()> {
        use crate::data::structures::{Contour, ContourKind, StructureRoi, StructureSet};
        let contour = Contour::new(
            ContourKind::ClosedPlanar,
            vec![
                DVec3::new(10., 0., 0.),
                DVec3::new(20., 0., 0.),
                DVec3::new(20., 10., 0.),
            ],
            vec![],
        )?;
        let set = StructureSet {
            sop_instance_uid: "synthetic".into(),
            label: "synthetic".into(),
            referenced_series: vec![],
            rois: vec![StructureRoi {
                number: 1,
                name: "triangle".into(),
                frame_of_reference_uid: "synthetic".into(),
                color: [1, 2, 3],
                contours: vec![contour],
            }],
        };
        let beam = BeamProjection::new(DVec3::ZERO, DVec3::X, DVec3::Y, DVec3::Z, 1000.)?;
        let projected = project_contours(&set, &beam, Some(0), 3)?;
        assert_eq!(projected[0].segments.len(), 3);
        assert_eq!(
            projected[0].segments[2],
            [DVec2::new(20., 10.), DVec2::new(10., 0.)]
        );
        assert!(project_contours(&set, &beam, Some(0), 2).is_err());
        let ap = BeamProjection::new(DVec3::ZERO, DVec3::X, DVec3::Z, -DVec3::Y, 1000.)?;
        assert_eq!(ap.edge_directions(), ["R", "L", "S", "I"]);
        Ok(())
    }
}
