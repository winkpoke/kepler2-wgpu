use super::*;
fn curve(units: DvhVolumeUnits) -> KeplerResult<DvhCurve> {
    DvhCurve::new(
        vec![DvhRoiReference {
            number: 1,
            contribution: DvhContribution::Included,
        }],
        DoseUnits::Gy,
        units,
        DvhStatistics {
            minimum: Some(10.),
            maximum: Some(40.),
            mean: Some(19.),
        },
        vec![[0., 100.], [10., 100.], [20., 50.], [40., 0.]],
    )
}
#[test]
fn queries_preserve_recorded_statistics_and_use_unsmoothed_bins() -> KeplerResult<()> {
    let c = curve(DvhVolumeUnits::Cm3)?;
    assert_eq!(c.statistics().mean, Some(19.));
    assert_eq!(c.volume_at(15.), Some(75.));
    assert_eq!(c.dose_at_percent(95.), Some(11.));
    assert!((c.dose_at_percent(2.).ok_or_else(|| invalid("D2"))? - 39.2).abs() < 1e-10);
    assert_eq!(c.dose_at_percent(100.), Some(10.));
    assert_eq!(c.volume_at(50.), Some(0.));
    assert_eq!(c.volume_cm3(), Some(100.));
    Ok(())
}
#[test]
fn percent_only_data_never_invents_absolute_volume() -> KeplerResult<()> {
    let c = curve(DvhVolumeUnits::Percent)?;
    assert_eq!(c.volume_cm3(), None);
    assert_eq!(c.percent_scale(), 1.);
    assert_eq!(c.dose_at_percent(95.), Some(11.));
    Ok(())
}
#[test]
fn truncated_tails_and_invalid_quantiles_are_unavailable() -> KeplerResult<()> {
    let c = DvhCurve::new(
        vec![DvhRoiReference {
            number: 1,
            contribution: DvhContribution::Included,
        }],
        DoseUnits::Gy,
        DvhVolumeUnits::Cm3,
        DvhStatistics::default(),
        vec![[5., 100.], [10., 40.]],
    )?;
    assert_eq!(c.volume_at(0.), None);
    assert_eq!(c.volume_at(11.), None);
    assert_eq!(c.dose_at_percent(2.), None);
    for v in [0., -1., 101., f64::NAN] {
        assert_eq!(c.dose_at_percent(v), None);
    }
    assert_eq!(c.dose_at_volume(f64::INFINITY), None);
    Ok(())
}
#[test]
fn nonmonotonic_nonfinite_and_ambiguous_histograms_are_rejected() {
    for points in [
        vec![],
        vec![[0., 0.]],
        vec![[0., 10.], [1., 11.]],
        vec![[0., 10.], [0., 0.]],
        vec![[0., f64::NAN]],
        vec![[0., f64::MAX]],
        vec![[f64::MAX, 1.]],
        vec![[0., 1e-320]],
    ] {
        assert!(DvhCurve::new(
            vec![DvhRoiReference {
                number: 1,
                contribution: DvhContribution::Included
            }],
            DoseUnits::Gy,
            DvhVolumeUnits::Cm3,
            DvhStatistics::default(),
            points
        )
        .is_err());
    }
}

#[test]
fn percent_curves_are_not_renormalized_when_the_first_bin_is_below_100() -> KeplerResult<()> {
    let c = DvhCurve::new(
        vec![DvhRoiReference {
            number: 1,
            contribution: DvhContribution::Included,
        }],
        DoseUnits::Gy,
        DvhVolumeUnits::Percent,
        DvhStatistics::default(),
        vec![[1., 75.], [10., 0.]],
    )?;
    assert_eq!(c.percent_scale(), 1.);
    assert_eq!(c.dose_at_percent(95.), None);
    Ok(())
}
#[test]
fn exact_structure_identity_roi_numbers_and_frame_are_required() -> KeplerResult<()> {
    use crate::data::structures::{StructureRoi, StructureSet};
    let source = DvhSet {
        source_dose_uid: "1.2.3".into(),
        structure_set_uid: Some("1.2.4".into()),
        frame_of_reference_uid: Some("1.2.5".into()),
        curves: vec![curve(DvhVolumeUnits::Cm3)?],
        issues: vec![],
    };
    let mut structures = StructureSet {
        sop_instance_uid: "1.2.4".into(),
        label: "Synthetic".into(),
        referenced_series: vec![],
        rois: vec![StructureRoi {
            number: 1,
            name: "Synthetic target".into(),
            frame_of_reference_uid: "1.2.5".into(),
            color: [0, 200, 0],
            contours: vec![],
        }],
    };
    source.validate_references(&structures)?;
    structures.sop_instance_uid = "1.2.99".into();
    assert!(source.validate_references(&structures).is_err());
    structures.sop_instance_uid = "1.2.4".into();
    structures.rois[0].number = 2;
    assert!(source.validate_references(&structures).is_err());
    structures.rois[0].number = 1;
    structures.rois[0].frame_of_reference_uid = "1.2.99".into();
    assert!(source.validate_references(&structures).is_err());
    Ok(())
}
