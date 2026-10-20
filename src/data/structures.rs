//! Patient-space structures, independent of graphics and filesystem access.

use glam::DVec3;

use crate::core::{KeplerError, KeplerResult};

/// Numerical coplanarity tolerance, in millimetres. This is not slice thickness.
pub const CONTOUR_PLANE_TOLERANCE_MM: f64 = 0.01;

/// DICOM contour semantics. XOR is retained even when drawing outlines only.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContourKind {
    Point,
    OpenPlanar,
    OpenNonplanar,
    ClosedPlanar,
    ClosedPlanarXor,
}

impl ContourKind {
    pub fn is_closed(self) -> bool {
        matches!(self, Self::ClosedPlanar | Self::ClosedPlanarXor)
    }
}

/// An original contour in DICOM patient LPS millimetres, without rasterization.
#[derive(Clone, Debug)]
pub struct Contour {
    kind: ContourKind,
    points: Vec<DVec3>,
    image_uids: Vec<String>,
    normal: Option<DVec3>,
}

impl Contour {
    /// Validate finite coordinates, cardinality and (where required) coplanarity.
    pub fn new(
        kind: ContourKind,
        points: Vec<DVec3>,
        image_uids: Vec<String>,
    ) -> KeplerResult<Self> {
        let invalid = |message: &str| KeplerError::Validation(message.to_owned());
        let minimum = if kind.is_closed() { 3 } else { 2 };
        if points.iter().any(|p| !p.is_finite())
            || (kind == ContourKind::Point && points.len() != 1)
            || (kind != ContourKind::Point && points.len() < minimum)
        {
            return Err(invalid("invalid contour coordinates or point count"));
        }
        let normal = if matches!(kind, ContourKind::Point | ContourKind::OpenNonplanar) {
            None
        } else {
            let origin = points[0];
            let axis = points
                .iter()
                .map(|p| *p - origin)
                .max_by(|a, b| a.length_squared().total_cmp(&b.length_squared()))
                .ok_or_else(|| invalid("empty contour"))?;
            let cross = points
                .iter()
                .map(|p| axis.cross(*p - origin))
                .max_by(|a, b| a.length_squared().total_cmp(&b.length_squared()))
                .ok_or_else(|| invalid("empty contour"))?;
            if cross.length() < 1e-10 {
                if kind.is_closed() {
                    return Err(invalid("degenerate closed contour"));
                }
                None
            } else {
                let normal = cross.normalize();
                if points
                    .iter()
                    .any(|p| normal.dot(*p - origin).abs() > CONTOUR_PLANE_TOLERANCE_MM)
                {
                    return Err(invalid("non-coplanar points in a planar contour"));
                }
                Some(normal)
            }
        };
        Ok(Self {
            kind,
            points,
            image_uids,
            normal,
        })
    }

    pub fn kind(&self) -> ContourKind {
        self.kind
    }
    /// Unit patient-space normal when the source contour defines a plane.
    pub fn normal(&self) -> Option<DVec3> {
        self.normal
    }

    pub fn points(&self) -> &[DVec3] {
        &self.points
    }
    pub fn image_uids(&self) -> &[String] {
        &self.image_uids
    }

    /// Only exact source-plane outlines are displayed. No slab or projection is inferred.
    pub fn lies_on(&self, plane: &StructurePlane) -> bool {
        self.kind != ContourKind::OpenNonplanar
            && self
                .normal
                .is_none_or(|n| n.dot(plane.normal()).abs() >= 1.0 - 1e-8)
            && self
                .points
                .iter()
                .all(|p| plane.distance(*p).abs() <= CONTOUR_PLANE_TOLERANCE_MM)
    }
}

pub use super::slice::SlicePlane as StructurePlane;

#[derive(Clone, Debug)]
pub struct StructureRoi {
    pub number: u32,
    pub name: String,
    pub frame_of_reference_uid: String,
    pub color: [u8; 3],
    pub contours: Vec<Contour>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReferencedSeries {
    pub frame_of_reference_uid: String,
    pub series_instance_uid: String,
    pub image_uids: Vec<String>,
}

/// Identity of the loaded CT, retained independently of display-axis reindexing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DicomVolumeIdentity {
    pub frame_of_reference_uid: String,
    pub series_instance_uid: String,
    pub image_uids: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct StructureSet {
    pub sop_instance_uid: String,
    pub label: String,
    pub referenced_series: Vec<ReferencedSeries>,
    pub rois: Vec<StructureRoi>,
}

impl StructureSet {
    /// Conservative source association; rejects other frames, series and missing referenced images.
    /// References may be absent in DICOM; in that case the ROI frame must still match.
    pub fn validate_reference(&self, image: &DicomVolumeIdentity) -> KeplerResult<()> {
        let fail = |message: &str| KeplerError::Validation(message.to_owned());
        if image.frame_of_reference_uid.is_empty()
            || image.series_instance_uid.is_empty()
            || self
                .rois
                .iter()
                .any(|r| r.frame_of_reference_uid != image.frame_of_reference_uid)
        {
            return Err(fail("structure and CT Frame of Reference do not match"));
        }
        let matching: Vec<_> = self
            .referenced_series
            .iter()
            .filter(|r| {
                r.frame_of_reference_uid == image.frame_of_reference_uid
                    && r.series_instance_uid == image.series_instance_uid
            })
            .collect();
        if !self.referenced_series.is_empty() && matching.is_empty() {
            return Err(fail("structure references a different CT series"));
        }
        let available: std::collections::HashSet<_> = image.image_uids.iter().collect();
        if matching
            .iter()
            .flat_map(|r| &r.image_uids)
            .chain(
                self.rois
                    .iter()
                    .flat_map(|r| &r.contours)
                    .flat_map(|c| &c.image_uids),
            )
            .any(|uid| !available.contains(uid))
        {
            return Err(fail(
                "structure references images absent from the loaded CT",
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn polygon(z: f64) -> Vec<DVec3> {
        vec![
            DVec3::new(0., 0., z),
            DVec3::new(10., 0., z),
            DVec3::new(10., 10., z),
            DVec3::new(0., 10., z),
        ]
    }
    #[test]
    fn contours_match_only_their_physical_plane() -> KeplerResult<()> {
        let contour = Contour::new(ContourKind::ClosedPlanarXor, polygon(4.), vec![])?;
        let mut plane = StructurePlane {
            origin: DVec3::new(-2., -2., 4.),
            right: DVec3::X,
            down: DVec3::Y,
            extent_mm: [20., 20.],
        };
        assert!(contour.lies_on(&plane));
        plane.origin.z += 0.1;
        assert!(!contour.lies_on(&plane));
        plane.origin.z = 4.;
        plane.right = DVec3::Z;
        assert!(!contour.lies_on(&plane));
        Ok(())
    }
    #[test]
    fn rejects_nonfinite_degenerate_and_nonplanar_closed_contours() {
        let mut points = polygon(0.);
        points[2].z = 1.;
        assert!(Contour::new(ContourKind::ClosedPlanar, points, vec![]).is_err());
        assert!(Contour::new(ContourKind::ClosedPlanar, vec![DVec3::ZERO; 3], vec![]).is_err());
        let mut points = polygon(0.);
        points[2].x = f64::NAN;
        assert!(Contour::new(ContourKind::ClosedPlanar, points, vec![]).is_err());
    }
}
