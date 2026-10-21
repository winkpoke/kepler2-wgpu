//! Optional DVH decoding from the already-decoded RTDOSE dataset.
use crate::{
    core::{KeplerError, KeplerResult},
    data::{dose::DoseUnits, dvh::*},
};
use dicom_object::InMemDicomObject;

fn invalid(message: &str) -> KeplerError {
    KeplerError::Dicom(message.into())
}
fn text(o: &InMemDicomObject, name: &str) -> KeplerResult<String> {
    o.element_by_name(name)
        .map_err(|_| invalid(&format!("missing {name}")))?
        .to_str()
        .map(|s| s.trim_matches([' ', '\0']).to_owned())
        .map_err(|_| invalid(&format!("invalid {name}")))
}
fn uid(o: &InMemDicomObject, name: &str) -> KeplerResult<String> {
    let value = text(o, name)?;
    if value.is_empty()
        || value.len() > 64
        || !value.bytes().all(|c| c.is_ascii_digit() || c == b'.')
    {
        return Err(invalid("invalid DVH identity"));
    }
    Ok(value)
}
fn items<'a>(o: &'a InMemDicomObject, name: &str) -> KeplerResult<&'a [InMemDicomObject]> {
    o.element_by_name(name)
        .map_err(|_| invalid(&format!("missing {name}")))?
        .items()
        .map(AsRef::as_ref)
        .ok_or_else(|| invalid(&format!("invalid {name}")))
}
fn integer(o: &InMemDicomObject, name: &str) -> KeplerResult<usize> {
    text(o, name)?
        .parse()
        .map_err(|_| invalid(&format!("invalid {name}")))
}
fn scalar(o: &InMemDicomObject, name: &str) -> KeplerResult<f64> {
    let values = o
        .element_by_name(name)
        .map_err(|_| invalid(&format!("missing {name}")))?
        .to_multi_float64()
        .map_err(|_| invalid(&format!("invalid {name}")))?;
    if values.len() != 1 || !values[0].is_finite() || values[0] < 0. {
        return Err(invalid(&format!("invalid {name}")));
    }
    Ok(values[0])
}
fn optional(o: &InMemDicomObject, name: &str) -> KeplerResult<Option<f64>> {
    if o.element_by_name(name).is_err() {
        Ok(None)
    } else {
        scalar(o, name).map(Some)
    }
}

/// Failures remain in DVH diagnostics so valid original dose pixels remain usable.
pub(super) fn read(o: &InMemDicomObject) -> Option<DvhSet> {
    o.element_by_name("DVHSequence").ok()?;
    let mut set = DvhSet::default();
    if let Err(e) = read_into(o, &mut set) {
        set.curves.clear();
        set.issues.push(DvhIssue {
            item: None,
            message: e.to_string(),
        });
    }
    Some(set)
}

fn read_into(o: &InMemDicomObject, set: &mut DvhSet) -> KeplerResult<()> {
    set.source_dose_uid = uid(o, "SOPInstanceUID")?;
    if o.element_by_name("FrameOfReferenceUID").is_ok() {
        set.frame_of_reference_uid = Some(uid(o, "FrameOfReferenceUID")?);
    }
    let structures = items(o, "ReferencedStructureSetSequence")?;
    if structures.len() != 1
        || uid(&structures[0], "ReferencedSOPClassUID")? != super::RTSTRUCT_SOP_CLASS_UID
    {
        return Err(invalid("DVH requires one referenced RT Structure Set"));
    }
    set.structure_set_uid = Some(uid(&structures[0], "ReferencedSOPInstanceUID")?);
    let curves = items(o, "DVHSequence")?;
    if curves.is_empty() || curves.len() > MAX_DVH_CURVES {
        return Err(invalid("DVH supports 1–256 curves"));
    }
    let mut bins = 0usize;
    for (index, o) in curves.iter().enumerate() {
        let result = (|| {
            let count = integer(o, "DVHNumberOfBins")?;
            bins = bins.saturating_add(count);
            if count == 0 || count > MAX_DVH_BINS || bins > MAX_DVH_TOTAL_BINS {
                return Err(invalid("DVH bin admission limit exceeded"));
            }
            curve(o, count)
        })();
        match result {
            Ok(curve) => set.curves.push(curve),
            Err(error) => set.issues.push(DvhIssue {
                item: Some(index + 1),
                message: error.to_string(),
            }),
        }
    }
    Ok(())
}

fn curve(o: &InMemDicomObject, count: usize) -> KeplerResult<DvhCurve> {
    if text(o, "DVHType")? != "CUMULATIVE" {
        return Err(invalid("only CUMULATIVE DVH is supported; curve skipped"));
    }
    if text(o, "DoseType")? != "PHYSICAL" {
        return Err(invalid("only PHYSICAL DVH is supported; curve skipped"));
    }
    let dose_units = match text(o, "DoseUnits")?.as_str() {
        "GY" => DoseUnits::Gy,
        "RELATIVE" => DoseUnits::Relative,
        _ => return Err(invalid("unsupported DVH dose units")),
    };
    let volume_units = match text(o, "DVHVolumeUnits")?.as_str() {
        "CM3" => DvhVolumeUnits::Cm3,
        "PERCENT" => DvhVolumeUnits::Percent,
        _ => return Err(invalid("unsupported DVH volume units")),
    };
    let scaling = scalar(o, "DVHDoseScaling")?;
    if scaling <= 0. {
        return Err(invalid("DVH dose scaling must be positive"));
    }
    let values = o
        .element_by_name("DVHData")
        .map_err(|_| invalid("missing DVH data"))?
        .to_multi_float64()
        .map_err(|_| invalid("invalid DVH data"))?;
    if values.len() != count * 2 {
        return Err(invalid("DVH bin count disagrees with data"));
    }
    let mut dose = 0.;
    let mut points = Vec::with_capacity(count);
    for (i, pair) in values.as_chunks::<2>().0.iter().enumerate() {
        let width = pair[0] * scaling;
        if !width.is_finite() || width < 0. || (i > 0 && width == 0.) {
            return Err(invalid("invalid DVH dose bin width"));
        }
        dose += width;
        points.push([dose, pair[1]]);
    }
    let references = items(o, "DVHReferencedROISequence")?;
    if references.len() > 64 {
        return Err(invalid("excessive DVH ROI contributions"));
    }
    let references = references
        .iter()
        .map(|r| {
            Ok(DvhRoiReference {
                number: u32::try_from(integer(r, "ReferencedROINumber")?)
                    .map_err(|_| invalid("invalid DVH ROI number"))?,
                contribution: match text(r, "DVHROIContributionType")?.as_str() {
                    "INCLUDED" => DvhContribution::Included,
                    "EXCLUDED" => DvhContribution::Excluded,
                    _ => return Err(invalid("unsupported DVH ROI contribution")),
                },
            })
        })
        .collect::<KeplerResult<Vec<_>>>()?;
    DvhCurve::new(
        references,
        dose_units,
        volume_units,
        DvhStatistics {
            minimum: optional(o, "DVHMinimumDose")?,
            maximum: optional(o, "DVHMaximumDose")?,
            mean: optional(o, "DVHMeanDose")?,
        },
        points,
    )
}
