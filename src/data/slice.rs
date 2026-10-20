//! Shared physical display-plane geometry in patient LPS millimeters.
use crate::core::{KeplerError, KeplerResult};
use glam::DVec3;
/// Physical image rectangle: top-left boundary, right/down unit directions and extent in mm.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SlicePlane {
    pub origin: DVec3,
    pub right: DVec3,
    pub down: DVec3,
    pub extent_mm: [f64; 2],
}

impl SlicePlane {
    pub fn validate(&self) -> KeplerResult<()> {
        if !self.origin.is_finite()
            || !self.right.is_finite()
            || !self.down.is_finite()
            || (self.right.length_squared() - 1.0).abs() > 1e-6
            || (self.down.length_squared() - 1.0).abs() > 1e-6
            || self.right.dot(self.down).abs() > 1e-6
            || self.extent_mm.iter().any(|v| !v.is_finite() || *v <= 0.0)
        {
            return Err(KeplerError::Validation(
                "invalid structure display plane".to_owned(),
            ));
        }
        Ok(())
    }
    pub fn normal(&self) -> DVec3 {
        self.right.cross(self.down)
    }
    pub fn distance(&self, point: DVec3) -> f64 {
        (point - self.origin).dot(self.normal())
    }
}
