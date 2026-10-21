//! Imported cumulative DVHs and bin-derived metrics, independent of images and rendering.
//!
//! Dose coordinates are in the declared source units. Volumes are cm³ or percent.
//! Queries interpolate linearly between recorded bins and never extrapolate an
//! unrecorded tail. Recorded statistics remain separate from bin-derived estimates.
use super::{dose::DoseUnits, structures::StructureSet};
use crate::core::{KeplerError, KeplerResult};
use std::sync::Arc;

/// Maximum number of retained histograms in one dose object.
pub const MAX_DVH_CURVES: usize = 256;
/// Maximum bins per curve and across the complete object, respectively.
pub const MAX_DVH_BINS: usize = 65_536;
pub const MAX_DVH_TOTAL_BINS: usize = 1_000_000;

/// Source volume units; percent-only histograms cannot supply absolute volumes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DvhVolumeUnits {
    Cm3,
    Percent,
}

/// A structure's contribution to a possibly composite histogram.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DvhContribution {
    Included,
    Excluded,
}

#[derive(Clone, Debug)]
pub struct DvhRoiReference {
    pub number: u32,
    pub contribution: DvhContribution,
}

/// Optional statistics supplied by the exporting system, not reconstructed from bins.
#[derive(Clone, Debug, Default)]
pub struct DvhStatistics {
    pub minimum: Option<f64>,
    pub maximum: Option<f64>,
    pub mean: Option<f64>,
}

/// A validated cumulative histogram. Original bin vertices are never smoothed.
#[derive(Clone, Debug)]
pub struct DvhCurve {
    references: Vec<DvhRoiReference>,
    dose_units: DoseUnits,
    volume_units: DvhVolumeUnits,
    statistics: DvhStatistics,
    points: Arc<[[f64; 2]]>,
}

fn invalid(message: &str) -> KeplerError {
    KeplerError::Validation(message.into())
}

impl DvhCurve {
    /// Validate cumulative [dose, volume] vertices in source units. At most 65,536
    /// strictly increasing dose coordinates, nonincreasing nonnegative volumes,
    /// and 1–64 unique ROI references are admitted. Values must fit the finite
    /// f32 numeric range, consistent with dose-grid admission; percentage scaling
    /// must remain finite. No patient data are logged.
    pub fn new(
        references: Vec<DvhRoiReference>,
        dose_units: DoseUnits,
        volume_units: DvhVolumeUnits,
        statistics: DvhStatistics,
        points: Vec<[f64; 2]>,
    ) -> KeplerResult<Self> {
        if references.is_empty()
            || references.len() > 64
            || !references
                .iter()
                .any(|r| r.contribution == DvhContribution::Included)
            || references
                .iter()
                .enumerate()
                .any(|(i, r)| references[..i].iter().any(|a| a.number == r.number))
        {
            return Err(invalid("invalid DVH ROI contributions"));
        }
        if points.is_empty()
            || points.len() > MAX_DVH_BINS
            || points.iter().any(|p| {
                p.iter()
                    .any(|v| !v.is_finite() || *v < 0. || *v > f64::from(f32::MAX))
            })
            || points
                .windows(2)
                .any(|p| p[1][0] <= p[0][0] || p[1][1] > p[0][1])
            || points[0][1] <= 0.
            || !(100. / points[0][1]).is_finite()
            || (volume_units == DvhVolumeUnits::Percent && points[0][1] > 100.)
        {
            return Err(invalid("invalid cumulative DVH bins"));
        }
        if [statistics.minimum, statistics.maximum, statistics.mean]
            .into_iter()
            .flatten()
            .any(|v| !v.is_finite() || v < 0.)
            || statistics
                .minimum
                .zip(statistics.maximum)
                .is_some_and(|(a, b)| a > b)
            || statistics
                .minimum
                .zip(statistics.mean)
                .is_some_and(|(a, b)| a > b)
            || statistics
                .mean
                .zip(statistics.maximum)
                .is_some_and(|(a, b)| a > b)
        {
            return Err(invalid("invalid recorded DVH statistics"));
        }
        Ok(Self {
            references,
            dose_units,
            volume_units,
            statistics,
            points: points.into(),
        })
    }
    pub fn references(&self) -> &[DvhRoiReference] {
        &self.references
    }
    pub fn dose_units(&self) -> DoseUnits {
        self.dose_units
    }
    pub fn volume_units(&self) -> DvhVolumeUnits {
        self.volume_units
    }
    pub fn statistics(&self) -> &DvhStatistics {
        &self.statistics
    }
    pub fn points(&self) -> &[[f64; 2]] {
        &self.points
    }
    /// Maximum recorded cumulative volume, in native volume units.
    pub fn initial_volume(&self) -> f64 {
        self.points[0][1]
    }
    /// Encoded histogram reference volume in cm³, unavailable for percent-only data.
    /// This is not an independently reconstructed ROI volume or a dose coverage measurement.
    pub fn volume_cm3(&self) -> Option<f64> {
        (self.volume_units == DvhVolumeUnits::Cm3).then(|| self.initial_volume())
    }
    /// Conversion multiplier from native volumes to percent of the histogram's reference volume.
    pub fn percent_scale(&self) -> f64 {
        match self.volume_units {
            DvhVolumeUnits::Cm3 => 100. / self.initial_volume(),
            DvhVolumeUnits::Percent => 1.,
        }
    }
    /// Vx in native volume units, using piecewise-linear bin interpolation.
    /// Missing tails return None unless a recorded extreme (or zero endpoint) proves them.
    pub fn volume_at(&self, dose: f64) -> Option<f64> {
        if !dose.is_finite() || dose < 0. {
            return None;
        }
        let first = self.points[0];
        let last = self.points[self.points.len() - 1];
        if dose < first[0] {
            return self
                .statistics
                .minimum
                .filter(|minimum| first[0] <= *minimum && dose <= *minimum)
                .map(|_| first[1]);
        }
        if dose > last[0] {
            return (last[1] == 0. || self.statistics.maximum.is_some_and(|m| m <= last[0]))
                .then_some(0.);
        }
        let i = self.points.partition_point(|p| p[0] <= dose);
        if i == self.points.len() {
            return Some(last[1]);
        }
        let a = self.points[i.saturating_sub(1)];
        let b = self.points[i];
        Some(a[1] + (b[1] - a[1]) * ((dose - a[0]) / (b[0] - a[0])))
    }
    /// Dx for a positive volume in native units. The right edge of a plateau is
    /// used; a truncated histogram cannot supply an unbracketed quantile.
    pub fn dose_at_volume(&self, volume: f64) -> Option<f64> {
        if !volume.is_finite() || volume <= 0. {
            return None;
        }
        let i = self.points.partition_point(|p| p[1] >= volume);
        if i == 0 {
            return None;
        }
        let a = self.points[i - 1];
        if i == self.points.len() {
            return (a[1] == volume).then_some(a[0]);
        }
        let b = self.points[i];
        Some(a[0] + (b[0] - a[0]) * ((a[1] - volume) / (a[1] - b[1])))
    }
    /// Dv% in source dose units; percent references the histogram's volume basis.
    pub fn dose_at_percent(&self, percent: f64) -> Option<f64> {
        if !percent.is_finite() || percent <= 0. || percent > 100. {
            return None;
        }
        self.dose_at_volume(percent / self.percent_scale())
    }
}

/// One skipped or invalid histogram, indexed using one-based source item numbering.
#[derive(Clone, Debug)]
pub struct DvhIssue {
    pub item: Option<usize>,
    pub message: String,
}

/// Imported source histograms. Their sampling/coverage method is not inferred.
#[derive(Clone, Debug, Default)]
pub struct DvhSet {
    pub source_dose_uid: String,
    pub structure_set_uid: Option<String>,
    pub frame_of_reference_uid: Option<String>,
    pub curves: Vec<DvhCurve>,
    pub issues: Vec<DvhIssue>,
}

impl DvhSet {
    /// Validate the exact structure-set identity and all referenced ROI numbers.
    /// CT and selected-plan association remain host workflow decisions.
    pub fn validate_references(&self, set: &StructureSet) -> KeplerResult<()> {
        if self.structure_set_uid.as_deref() != Some(set.sop_instance_uid.as_str()) {
            return Err(invalid("DVH references a different structure set"));
        }
        for reference in self.curves.iter().flat_map(|c| c.references()) {
            let roi = set
                .rois
                .iter()
                .find(|r| r.number == reference.number)
                .ok_or_else(|| invalid("DVH references an ROI absent from the structure set"))?;
            if self
                .frame_of_reference_uid
                .as_ref()
                .is_some_and(|f| *f != roi.frame_of_reference_uid)
            {
                return Err(invalid("DVH dose and ROI frames differ"));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
