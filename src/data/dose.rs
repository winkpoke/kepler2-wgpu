//! Quantitative dose grids, independent of CT storage and display encoding.
use crate::core::{KeplerError, KeplerResult};
use glam::DVec3;

pub const MAX_DOSE_VOXELS: usize = 64 * 1024 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DoseUnits {
    Gy,
    Relative,
}
impl DoseUnits {
    pub fn label(self) -> &'static str {
        match self {
            Self::Gy => "Gy",
            Self::Relative => "relative",
        }
    }
}
/// Original integer samples. Display conversion never replaces these values.
#[derive(Debug)]
pub enum DoseSamples {
    U16(Vec<u16>),
    U32(Vec<u32>),
}
impl DoseSamples {
    pub fn len(&self) -> usize {
        match self {
            Self::U16(v) => v.len(),
            Self::U32(v) => v.len(),
        }
    }
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
    fn at(&self, i: usize) -> f64 {
        match self {
            Self::U16(v) => f64::from(v[i]),
            Self::U32(v) => f64::from(v[i]),
        }
    }
    pub(crate) fn reverse_frames(&mut self, stride: usize) {
        match self {
            Self::U16(v) => reverse_frames(v, stride),
            Self::U32(v) => reverse_frames(v, stride),
        }
    }
}
fn reverse_frames<T>(v: &mut [T], stride: usize) {
    let frames = v.len() / stride;
    for z in 0..frames / 2 {
        for i in 0..stride {
            v.swap(z * stride + i, (frames - 1 - z) * stride + i);
        }
    }
}
#[derive(Clone, Debug)]
pub struct DoseMetadata {
    pub sop_instance_uid: String,
    pub frame_of_reference_uid: String,
    pub units: DoseUnits,
    pub summation_type: String,
    pub referenced_plan_uids: Vec<String>,
}
/// Patient LPS mm. `column` and `row` are orthonormal directions; spacing is [column,row].
/// Offsets are strictly increasing along column × row, relative to `origin`.
#[derive(Clone, Debug)]
pub struct DoseGeometry {
    pub origin: DVec3,
    pub column: DVec3,
    pub row: DVec3,
    pub spacing: [f64; 2],
    pub frame_offsets: Vec<f64>,
}
#[derive(Debug)]
pub struct DoseGrid {
    metadata: DoseMetadata,
    geometry: DoseGeometry,
    dimensions: [usize; 3],
    samples: DoseSamples,
    scaling: f64,
    maximum: f64,
    dvhs: Option<super::dvh::DvhSet>,
}
fn invalid(message: &str) -> KeplerError {
    KeplerError::Validation(message.into())
}
impl DoseGrid {
    /// Validates one physical multiframe grid; no resampling, integer clipping or normalization.
    pub fn new(
        metadata: DoseMetadata,
        geometry: DoseGeometry,
        dimensions: [usize; 3],
        samples: DoseSamples,
        scaling: f64,
    ) -> KeplerResult<Self> {
        let count = dimensions
            .into_iter()
            .try_fold(1usize, usize::checked_mul)
            .ok_or_else(|| invalid("dose dimensions overflow"))?;
        if dimensions.contains(&0)
            || dimensions[2] < 2
            || count > MAX_DOSE_VOXELS
            || samples.len() != count
        {
            return Err(invalid(
                "invalid or excessive dose dimensions/sample count; multiframe grid required",
            ));
        }
        if !scaling.is_finite() || scaling <= 0. || metadata.frame_of_reference_uid.is_empty() {
            return Err(invalid("invalid dose scaling or Frame of Reference"));
        }
        if !geometry.origin.is_finite()
            || !geometry.column.is_finite()
            || !geometry.row.is_finite()
            || (geometry.column.length_squared() - 1.).abs() > 1e-6
            || (geometry.row.length_squared() - 1.).abs() > 1e-6
            || geometry.column.dot(geometry.row).abs() > 1e-6
            || geometry.spacing.iter().any(|s| !s.is_finite() || *s <= 0.)
            || geometry.frame_offsets.len() != dimensions[2]
            || geometry.frame_offsets.iter().any(|v| !v.is_finite())
            || geometry.frame_offsets.windows(2).any(|w| w[0] >= w[1])
        {
            return Err(invalid(
                "invalid dose orientation, spacing or frame offsets",
            ));
        }
        let maximum = (0..samples.len())
            .map(|i| samples.at(i) * scaling)
            .fold(0f64, f64::max);
        if !maximum.is_finite() || maximum > f64::from(f32::MAX) {
            return Err(invalid("dose exceeds display numeric range"));
        }
        Ok(Self {
            metadata,
            geometry,
            dimensions,
            samples,
            scaling,
            maximum,
            dvhs: None,
        })
    }
    pub fn metadata(&self) -> &DoseMetadata {
        &self.metadata
    }
    pub fn geometry(&self) -> &DoseGeometry {
        &self.geometry
    }
    pub fn dimensions(&self) -> [usize; 3] {
        self.dimensions
    }
    pub fn samples(&self) -> &DoseSamples {
        &self.samples
    }
    pub fn scaling(&self) -> f64 {
        self.scaling
    }
    pub fn maximum(&self) -> f64 {
        self.maximum
    }
    /// Imported source DVHs and their diagnostics, if present. These are not recalculated.
    pub fn dvhs(&self) -> Option<&super::dvh::DvhSet> {
        self.dvhs.as_ref()
    }
    pub(crate) fn with_dvhs(mut self, dvhs: Option<super::dvh::DvhSet>) -> Self {
        self.dvhs = dvhs;
        self
    }
    pub fn gpu_bytes(&self) -> usize {
        self.samples.len() * 4
    }
    pub fn local_position(&self, patient: DVec3) -> DVec3 {
        let delta = patient - self.geometry.origin;
        DVec3::new(
            delta.dot(self.geometry.column) / self.geometry.spacing[0],
            delta.dot(self.geometry.row) / self.geometry.spacing[1],
            delta.dot(self.geometry.column.cross(self.geometry.row)),
        )
    }
    /// One quantitative point, interpolated from original samples in f64.
    /// Outside the sample-center domain returns None; it never means zero dose.
    pub fn sample_lps(&self, patient: DVec3) -> Option<f64> {
        if !patient.is_finite() {
            return None;
        }
        let p = self.local_position(patient);
        let [nx, ny, nz] = self.dimensions;
        let offsets = &self.geometry.frame_offsets;
        if p.x < 0.
            || p.y < 0.
            || p.x > (nx - 1) as f64
            || p.y > (ny - 1) as f64
            || p.z < offsets[0]
            || p.z > offsets[nz - 1]
        {
            return None;
        }
        let z1 = offsets.partition_point(|z| *z <= p.z).clamp(1, nz - 1);
        let z0 = z1 - 1;
        let t = (p.z - offsets[z0]) / (offsets[z1] - offsets[z0]);
        let x0 = p.x.floor() as usize;
        let x1 = (x0 + 1).min(nx - 1);
        let y0 = p.y.floor() as usize;
        let y1 = (y0 + 1).min(ny - 1);
        let tx = p.x - x0 as f64;
        let ty = p.y - y0 as f64;
        let layer = |z| {
            let at = |x, y| self.samples.at((z * ny + y) * nx + x) * self.scaling;
            let a = at(x0, y0) * (1. - tx) + at(x1, y0) * tx;
            let b = at(x0, y1) * (1. - tx) + at(x1, y1) * tx;
            a * (1. - ty) + b * ty
        };
        Some(layer(z0) * (1. - t) + layer(z1) * t)
    }
    /// Approximate f32 display copy only. Original integer/scaling data remain authoritative.
    pub fn display_values(&self) -> Vec<f32> {
        (0..self.samples.len())
            .map(|i| (self.samples.at(i) * self.scaling) as f32)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn dose_interpolation_uses_physical_offsets_and_preserves_u32() -> KeplerResult<()> {
        let origin = DVec3::new(1e8, -2e8, 3e8);
        let geometry = DoseGeometry {
            origin,
            column: DVec3::Y,
            row: DVec3::Z,
            spacing: [2., 3.],
            frame_offsets: vec![0., 2., 7.],
        };
        let samples = DoseSamples::U32(
            (0..3)
                .flat_map(|z| {
                    (0..2)
                        .flat_map(move |y| (0..2).map(move |x| 100000 + 100 * z + 20 * y + 10 * x))
                })
                .collect(),
        );
        let grid = DoseGrid::new(
            DoseMetadata {
                sop_instance_uid: "1.2.3".into(),
                frame_of_reference_uid: "1.2.4".into(),
                units: DoseUnits::Gy,
                summation_type: "PLAN".into(),
                referenced_plan_uids: vec![],
            },
            geometry,
            [2, 2, 3],
            samples,
            0.001,
        )?;
        assert!(
            (grid
                .sample_lps(origin + DVec3::new(4.5, 1., 1.5))
                .ok_or_else(|| invalid("sample"))?
                - 100.165)
                .abs()
                < 1e-8
        );
        assert!(grid.sample_lps(origin - DVec3::Y).is_none());
        assert!(matches!(grid.samples(),DoseSamples::U32(v) if v[0]==100000));
        assert_eq!(grid.scaling(), 0.001);
        let mut g = grid.geometry().clone();
        g.frame_offsets[2] = 2.;
        assert!(DoseGrid::new(
            grid.metadata().clone(),
            g,
            [2, 2, 3],
            DoseSamples::U16(vec![0; 12]),
            1.
        )
        .is_err());
        Ok(())
    }
}
