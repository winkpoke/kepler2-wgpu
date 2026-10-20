//! Bounded RT Structure Set import through the existing DICOM decoder.

use super::CTImage;
use crate::{
    core::{KeplerError, KeplerResult},
    data::structures::*,
};
use dicom_object::{FileDicomObject, InMemDicomObject};
use glam::DVec3;
use std::collections::{BTreeMap, HashSet};

pub const RTSTRUCT_SOP_CLASS_UID: &str = "1.2.840.10008.5.1.4.1.1.481.3";
pub const MAX_RTSTRUCT_BYTES: usize = 32 * 1024 * 1024;
pub const MAX_RTSTRUCT_POINTS: usize = 500_000;
const MAX_ROIS: usize = 4096;
const MAX_CONTOURS: usize = 50_000;

fn invalid(message: &str) -> KeplerError {
    KeplerError::Dicom(message.to_owned())
}
fn text(obj: &InMemDicomObject, name: &str) -> KeplerResult<String> {
    obj.element_by_name(name)
        .ok()
        .and_then(|e| e.to_str().ok())
        .map(|v| v.trim().to_owned())
        .ok_or_else(|| invalid(&format!("missing or invalid {name}")))
}
fn uid(obj: &InMemDicomObject, name: &str) -> KeplerResult<String> {
    let value = text(obj, name)?;
    if value.is_empty()
        || value.len() > 64
        || !value.bytes().all(|c| c.is_ascii_digit() || c == b'.')
    {
        return Err(invalid(&format!("invalid {name}")));
    }
    Ok(value)
}
fn items<'a>(obj: &'a InMemDicomObject, name: &str) -> KeplerResult<&'a [InMemDicomObject]> {
    obj.element_by_name(name)
        .ok()
        .and_then(|e| e.items())
        .ok_or_else(|| invalid(&format!("missing or invalid {name}")))
}
fn optional_items<'a>(
    obj: &'a InMemDicomObject,
    name: &str,
) -> KeplerResult<&'a [InMemDicomObject]> {
    if obj.element_by_name(name).is_err() {
        Ok(&[])
    } else {
        items(obj, name)
    }
}
fn number(obj: &InMemDicomObject, name: &str) -> KeplerResult<u32> {
    text(obj, name)?
        .parse()
        .map_err(|_| invalid(&format!("invalid {name}")))
}
fn image_uids(obj: &InMemDicomObject) -> KeplerResult<Vec<String>> {
    optional_items(obj, "ContourImageSequence")?
        .iter()
        .map(|i| uid(i, "ReferencedSOPInstanceUID"))
        .collect()
}

/// Identify an RT Structure Set by SOP Class, independent of filename extension.
/// Callers must bound input before reading it; this classifier never loads a directory.
pub fn is_rtstruct(bytes: &[u8]) -> bool {
    bytes.len() <= MAX_RTSTRUCT_BYTES
        && FileDicomObject::from_reader(bytes)
            .ok()
            .and_then(|obj| text(&obj, "SOPClassUID").ok())
            .is_some_and(|class| class == RTSTRUCT_SOP_CLASS_UID)
}

/// Decode a CT and its optional frame identity in a single DICOM parse.
/// Existing CTImage constructors remain source-compatible.
pub fn read_ct_image_with_frame(bytes: &[u8]) -> KeplerResult<(CTImage, Option<String>)> {
    let obj = FileDicomObject::from_reader(bytes).map_err(|_| invalid("cannot decode CT DICOM"))?;
    let ct_class = "1.2.840.10008.5.1.4.1.1.2";
    // Preserve the existing reader's support for files whose class is only in file meta.
    if obj.meta().media_storage_sop_class_uid() != ct_class
        || (obj.element_by_name("SOPClassUID").is_ok() && text(&obj, "SOPClassUID")? != ct_class)
    {
        return Err(invalid("file is not a classic CT image"));
    }
    let image = CTImage::from_dicom_object(&obj).map_err(|_| invalid("cannot decode CT image"))?;
    // A bad/missing identity disables association without making the CT unreadable.
    let frame = uid(&obj, "FrameOfReferenceUID").ok();
    Ok((image, frame))
}

/// Import original LPS coordinates and references; never constructs a volume or GPU resource.
/// Input bytes, ROI count, contour count and aggregate points have fixed limits.
pub fn read_rtstruct(bytes: &[u8]) -> KeplerResult<StructureSet> {
    if bytes.len() > MAX_RTSTRUCT_BYTES {
        return Err(invalid("RTSTRUCT exceeds 32 MiB input limit"));
    }
    let obj =
        FileDicomObject::from_reader(bytes).map_err(|_| invalid("cannot decode RTSTRUCT DICOM"))?;
    if text(&obj, "SOPClassUID")? != RTSTRUCT_SOP_CLASS_UID {
        return Err(invalid("file is not an RT Structure Set"));
    }
    let mut referenced_series = Vec::new();
    for frame in optional_items(&obj, "ReferencedFrameOfReferenceSequence")? {
        let frame_uid = uid(frame, "FrameOfReferenceUID")?;
        for study in optional_items(frame, "RTReferencedStudySequence")? {
            for series in items(study, "RTReferencedSeriesSequence")? {
                referenced_series.push(ReferencedSeries {
                    frame_of_reference_uid: frame_uid.clone(),
                    series_instance_uid: uid(series, "SeriesInstanceUID")?,
                    image_uids: image_uids(series)?,
                });
            }
        }
    }
    let definitions = items(&obj, "StructureSetROISequence")?;
    if definitions.is_empty() || definitions.len() > MAX_ROIS {
        return Err(invalid("invalid or excessive ROI count"));
    }
    let mut rois = BTreeMap::new();
    for item in definitions {
        let number = number(item, "ROINumber")?;
        let roi = StructureRoi {
            number,
            name: text(item, "ROIName")?,
            frame_of_reference_uid: uid(item, "ReferencedFrameOfReferenceUID")?,
            color: [255, 180, 0],
            contours: Vec::new(),
        };
        if rois.insert(number, roi).is_some() {
            return Err(invalid("duplicate ROI Number"));
        }
    }
    let mut seen = HashSet::new();
    let mut total_points = 0usize;
    let mut total_contours = 0usize;
    for roi_item in optional_items(&obj, "ROIContourSequence")? {
        let id = number(roi_item, "ReferencedROINumber")?;
        if !seen.insert(id) {
            return Err(invalid("duplicate ROI contour entry"));
        }
        let roi = rois
            .get_mut(&id)
            .ok_or_else(|| invalid("contours reference an undefined ROI"))?;
        if roi_item.element_by_name("ROIDisplayColor").is_ok() {
            let color = text(roi_item, "ROIDisplayColor")?
                .split('\\')
                .map(|s| s.trim().parse::<u8>())
                .collect::<Result<Vec<_>, _>>()
                .map_err(|_| invalid("invalid ROI display color"))?;
            roi.color = color
                .try_into()
                .map_err(|_| invalid("ROI color must contain three values"))?;
        }
        for item in optional_items(roi_item, "ContourSequence")? {
            total_contours += 1;
            let count = number(item, "NumberOfContourPoints")? as usize;
            total_points = total_points
                .checked_add(count)
                .ok_or_else(|| invalid("point count overflow"))?;
            if total_points > MAX_RTSTRUCT_POINTS || total_contours > MAX_CONTOURS {
                return Err(invalid("RTSTRUCT exceeds contour/point budget"));
            }
            let kind = match text(item, "ContourGeometricType")?.as_str() {
                "POINT" => ContourKind::Point,
                "OPEN_PLANAR" => ContourKind::OpenPlanar,
                "OPEN_NONPLANAR" => ContourKind::OpenNonplanar,
                "CLOSED_PLANAR" => ContourKind::ClosedPlanar,
                "CLOSEDPLANAR_XOR" => ContourKind::ClosedPlanarXor,
                _ => return Err(invalid("unsupported contour geometric type")),
            };
            let element = item
                .element_by_name("ContourData")
                .map_err(|_| invalid("missing Contour Data"))?;
            // PS3.3 C.8.8.6 permits UN for large DS payloads in Explicit VR.
            let data = if element.vr() == dicom_core::VR::UN {
                let bytes = element
                    .to_bytes()
                    .map_err(|_| invalid("invalid UN Contour Data"))?;
                std::str::from_utf8(&bytes)
                    .map_err(|_| invalid("invalid UN Contour Data"))?
                    .trim_end_matches('\0')
                    .split('\\')
                    .map(|value| {
                        value
                            .trim()
                            .parse::<f64>()
                            .map_err(|_| invalid("invalid UN Contour Data"))
                    })
                    .collect::<KeplerResult<Vec<_>>>()?
            } else {
                element
                    .to_multi_float64()
                    .map_err(|_| invalid("invalid Contour Data"))?
            };
            if data.len()
                != count
                    .checked_mul(3)
                    .ok_or_else(|| invalid("point count overflow"))?
            {
                return Err(invalid(
                    "Contour Data does not match Number of Contour Points",
                ));
            }
            let points = data
                .as_chunks::<3>()
                .0
                .iter()
                .map(|p| DVec3::new(p[0], p[1], p[2]))
                .collect();
            roi.contours
                .push(Contour::new(kind, points, image_uids(item)?)?);
        }
        if roi
            .contours
            .iter()
            .any(|c| c.kind() == ContourKind::ClosedPlanarXor)
            && roi
                .contours
                .iter()
                .any(|c| c.kind() != ContourKind::ClosedPlanarXor)
        {
            return Err(invalid(
                "XOR ROI must contain only CLOSEDPLANAR_XOR contours",
            ));
        }
    }
    Ok(StructureSet {
        sop_instance_uid: uid(&obj, "SOPInstanceUID")?,
        label: text(&obj, "StructureSetLabel")?,
        referenced_series,
        rois: rois.into_values().collect(),
    })
}

#[cfg(test)]
#[path = "rtstruct_tests.rs"]
mod tests;
