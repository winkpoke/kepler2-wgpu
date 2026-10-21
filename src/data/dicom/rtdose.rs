//! Bounded physical multiframe RTDOSE import using the existing DICOM decoder.
use crate::{
    core::{KeplerError, KeplerResult},
    data::dose::*,
};
use dicom_object::{FileDicomObject, InMemDicomObject};
use glam::DVec3;

pub const RTDOSE_SOP_CLASS_UID: &str = "1.2.840.10008.5.1.4.1.1.481.2";
pub const MAX_RTDOSE_BYTES: usize = 256 * 1024 * 1024;
fn invalid(s: &str) -> KeplerError {
    KeplerError::Dicom(s.into())
}
fn text(o: &InMemDicomObject, name: &str) -> KeplerResult<String> {
    o.element_by_name(name)
        .ok()
        .and_then(|e| e.to_str().ok())
        .map(|s| s.trim().into())
        .ok_or_else(|| invalid(&format!("missing or invalid {name}")))
}
fn uid(o: &InMemDicomObject, name: &str) -> KeplerResult<String> {
    let s = text(o, name)?;
    if s.is_empty() || s.len() > 64 || !s.bytes().all(|c| c.is_ascii_digit() || c == b'.') {
        return Err(invalid(&format!("invalid {name}")));
    }
    Ok(s)
}
fn values(o: &InMemDicomObject, name: &str) -> KeplerResult<Vec<f64>> {
    o.element_by_name(name)
        .map_err(|_| invalid(&format!("missing {name}")))?
        .to_multi_float64()
        .map_err(|_| invalid(&format!("invalid {name}")))
}
fn integer(o: &InMemDicomObject, name: &str) -> KeplerResult<usize> {
    text(o, name)?
        .parse()
        .map_err(|_| invalid(&format!("invalid {name}")))
}
/// Read just the SOP class from a bounded prefix/file reader, stopping before later data.
/// The caller owns filesystem access and can use this before allocating the full input.
pub fn rt_sop_class(reader: impl std::io::Read) -> KeplerResult<String> {
    let obj = dicom_object::OpenFileOptions::new()
        .read_until(dicom_core::Tag(0x0008, 0x0018))
        .from_reader(reader)
        .map_err(|_| invalid("cannot read DICOM SOP class"))?;
    text(&obj, "SOPClassUID").or_else(|_| Ok(obj.meta().media_storage_sop_class_uid().into()))
}
/// Supports uncompressed LE/BE 16/32-bit unsigned PHYSICAL grids in Gy or relative units.
/// Single-frame, compressed, signed error and coded/effective doses return explicit errors.
pub fn read_rt_dose(bytes: &[u8]) -> KeplerResult<DoseGrid> {
    if bytes.len() > MAX_RTDOSE_BYTES {
        return Err(invalid("RTDOSE exceeds 256 MiB input limit"));
    }
    let o =
        FileDicomObject::from_reader(bytes).map_err(|_| invalid("cannot decode RTDOSE DICOM"))?;
    if text(&o, "SOPClassUID")? != RTDOSE_SOP_CLASS_UID
        || o.meta().media_storage_sop_class_uid() != RTDOSE_SOP_CLASS_UID
    {
        return Err(invalid("file is not an RT Dose"));
    }
    let big_endian = match o.meta().transfer_syntax() {
        "1.2.840.10008.1.2" | "1.2.840.10008.1.2.1" => false,
        "1.2.840.10008.1.2.2" => true,
        _ => return Err(invalid("compressed RTDOSE transfer syntax is unsupported")),
    };
    if text(&o, "DoseType")? != "PHYSICAL" {
        return Err(invalid(
            "only PHYSICAL RTDOSE is supported; effective, coded and signed error doses are unsupported",
        ));
    }
    let units = match text(&o, "DoseUnits")?.as_str() {
        "GY" => DoseUnits::Gy,
        "RELATIVE" => DoseUnits::Relative,
        _ => return Err(invalid("coded RTDOSE units are unsupported")),
    };
    if integer(&o, "SamplesPerPixel")? != 1
        || text(&o, "PhotometricInterpretation")? != "MONOCHROME2"
        || integer(&o, "PixelRepresentation")? != 0
    {
        return Err(invalid("dose requires unsigned monochrome samples"));
    }
    let bits = integer(&o, "BitsAllocated")?;
    if ![16, 32].contains(&bits)
        || integer(&o, "BitsStored")? != bits
        || integer(&o, "HighBit")? != bits - 1
    {
        return Err(invalid("invalid RTDOSE pixel bit layout"));
    }
    let dimensions = [
        integer(&o, "Columns")?,
        integer(&o, "Rows")?,
        integer(&o, "NumberOfFrames")?,
    ];
    let count = dimensions
        .into_iter()
        .try_fold(1usize, usize::checked_mul)
        .ok_or_else(|| invalid("dose dimensions overflow"))?;
    if dimensions.contains(&0) || dimensions[2] < 2 || count > MAX_DOSE_VOXELS {
        return Err(invalid(
            "invalid or excessive dose dimensions; multiframe grid required",
        ));
    }
    let pointer = o
        .element_by_name("FrameIncrementPointer")
        .map_err(|_| invalid("missing Frame Increment Pointer"))?;
    if !matches!(pointer.value().primitive(),Some(dicom_core::PrimitiveValue::Tags(tags)) if tags.as_slice()==[dicom_core::Tag(0x3004,0x000c)])
    {
        return Err(invalid(
            "dose Frame Increment Pointer must identify Grid Frame Offset Vector",
        ));
    }
    let origin: DVec3 = DVec3::from_array(
        values(&o, "ImagePositionPatient")?
            .try_into()
            .map_err(|_| invalid("invalid dose origin"))?,
    );
    let orientation = values(&o, "ImageOrientationPatient")?;
    if orientation.len() != 6 {
        return Err(invalid("invalid dose orientation"));
    }
    let column = DVec3::from_slice(&orientation[..3]);
    let row = DVec3::from_slice(&orientation[3..]);
    let spacing: [f64; 2] = values(&o, "PixelSpacing")?
        .try_into()
        .map_err(|_| invalid("invalid dose PixelSpacing"))?;
    let mut offsets = values(&o, "GridFrameOffsetVector")?;
    if offsets.len() != dimensions[2] || offsets.iter().any(|v| !v.is_finite()) {
        return Err(invalid("dose frame offsets must match Number of Frames"));
    }
    // Normalize both DICOM conventions to distances along column × row from IPP.
    if offsets[0].abs() > 1e-6 {
        if (column - DVec3::X).length() > 1e-6
            || (row - DVec3::Y).length() > 1e-6
            || (offsets[0] - origin.z).abs() > 1e-6
        {
            return Err(invalid("unsupported dose frame-offset interpretation"));
        }
        for z in &mut offsets {
            *z -= origin.z;
        }
    }
    let ascending = offsets.windows(2).all(|w| w[1] > w[0]);
    let descending = offsets.windows(2).all(|w| w[1] < w[0]);
    if !ascending && !descending {
        return Err(invalid("dose frame offsets must be strictly monotonic"));
    }
    let scaling = values(&o, "DoseGridScaling")?;
    if scaling.len() != 1 {
        return Err(invalid("invalid Dose Grid Scaling"));
    }
    let element = o
        .element_by_name("PixelData")
        .map_err(|_| invalid("dose has no pixel grid"))?;
    let data = element
        .to_bytes()
        .map_err(|_| invalid("invalid dose Pixel Data"))?;
    if data.len() != count * (bits / 8) {
        return Err(invalid("dose pixel length does not match dimensions"));
    }
    // OW primitives are normalized by dicom-rs; use typed words rather than reinterpreting host bytes.
    let mut samples = if bits == 16 {
        DoseSamples::U16(
            element
                .to_multi_int::<u16>()
                .map_err(|_| invalid("invalid 16-bit dose pixels"))?,
        )
    } else {
        // OW is a stream of 16-bit words, also for 32-bit RT Dose samples.
        let words = element
            .to_multi_int::<u16>()
            .map_err(|_| invalid("invalid 32-bit dose pixel words"))?;
        DoseSamples::U32(
            words
                .as_chunks::<2>()
                .0
                .iter()
                .map(|w| {
                    if big_endian {
                        (u32::from(w[0]) << 16) | u32::from(w[1])
                    } else {
                        u32::from(w[0]) | (u32::from(w[1]) << 16)
                    }
                })
                .collect(),
        )
    };
    if descending {
        offsets.reverse();
        samples.reverse_frames(dimensions[0] * dimensions[1]);
    }
    let mut plans = Vec::new();
    if let Ok(e) = o.element_by_name("ReferencedRTPlanSequence") {
        for item in e
            .items()
            .ok_or_else(|| invalid("invalid Referenced RT Plan Sequence"))?
        {
            plans.push(uid(item, "ReferencedSOPInstanceUID")?);
        }
    }
    let summation_type = text(&o, "DoseSummationType")?;
    let known = [
        "PLAN",
        "MULTI_PLAN",
        "PLAN_OVERVIEW",
        "FRACTION",
        "BEAM",
        "BRACHY",
        "FRACTION_SESSION",
        "BEAM_SESSION",
        "BRACHY_SESSION",
        "CONTROL_POINT",
        "RECORD",
    ];
    if !known.contains(&summation_type.as_str()) {
        return Err(invalid("unsupported Dose Summation Type"));
    }
    if !["PLAN_OVERVIEW", "RECORD"].contains(&summation_type.as_str()) && plans.is_empty() {
        return Err(invalid("dose is missing referenced RT Plan"));
    }
    let dvhs = super::rtdvh::read(&o);
    DoseGrid::new(
        DoseMetadata {
            sop_instance_uid: uid(&o, "SOPInstanceUID")?,
            frame_of_reference_uid: uid(&o, "FrameOfReferenceUID")?,
            units,
            summation_type,
            referenced_plan_uids: plans,
        },
        DoseGeometry {
            origin,
            column,
            row,
            spacing: [spacing[1], spacing[0]],
            frame_offsets: offsets,
        },
        dimensions,
        samples,
        scaling[0],
    )
    .map(|grid| grid.with_dvhs(dvhs))
}
#[cfg(test)]
#[path = "rtdose_tests.rs"]
mod tests;
