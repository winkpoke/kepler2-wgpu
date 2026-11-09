//! Import of one classic, uncompressed MR stack.
use std::collections::BTreeSet;

use dicom_object::{FileDicomObject, FileMetaTable, InMemDicomObject};
use glam::{DVec3, Mat4};

use crate::{
    core::{coord::Base, KeplerError, KeplerResult},
    data::{ct_volume::CTVolume, structures::DicomVolumeIdentity},
};

pub const MR_IMAGE_STORAGE: &str = "1.2.840.10008.5.1.4.1.1.4";
const MAX_BYTES: usize = 1024 * 1024 * 1024;
const MAX_VOXELS: usize = 128 * 1024 * 1024;
const MAX_SLICES: usize = 8192;
const ORIENTATION_TOLERANCE: f64 = 1e-4;
const POSITION_TOLERANCE_MM: f64 = 0.01;

fn invalid(message: &str) -> KeplerError {
    KeplerError::Dicom(format!("MR: {message}"))
}

fn text(o: &InMemDicomObject, key: &str) -> Option<String> {
    Some(
        o.element_by_name(key)
            .ok()?
            .to_str()
            .ok()?
            .trim()
            .to_owned(),
    )
}

fn number<T: std::str::FromStr>(o: &InMemDicomObject, key: &str) -> Option<T> {
    text(o, key)?.parse().ok()
}

fn uid(o: &InMemDicomObject, key: &str) -> KeplerResult<String> {
    text(o, key)
        .filter(|s| {
            s.len() <= 64
                && s.split('.')
                    .all(|p| !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit()))
        })
        .ok_or_else(|| invalid(&format!("missing or invalid {key}")))
}

fn values<const N: usize>(o: &InMemDicomObject, key: &str) -> KeplerResult<[f64; N]> {
    o.element_by_name(key)
        .ok()
        .and_then(|e| e.to_multi_float64().ok())
        .and_then(|v| <[f64; N]>::try_from(v).ok())
        .filter(|v| v.iter().all(|x| x.is_finite()))
        .ok_or_else(|| invalid(&format!("missing or invalid {key}")))
}

#[derive(PartialEq)]
struct Grid {
    dimensions: [usize; 2],
    spacing: [f64; 2],
    series: String,
    frame: String,
    acquisition: [Option<String>; 3],
}

struct Slice {
    uid: String,
    origin: DVec3,
    x: DVec3,
    y: DVec3,
    single_spacing: Option<f64>,
    window_level: Option<[f32; 2]>,
    pixels: Vec<i16>,
}

/// A resident scalar volume and its original DICOM coordinates and identity.
pub struct ImportedMrSeries {
    pub volume: CTVolume,
    /// Voxel centres to DICOM LPS millimetres, column major.
    pub patient_lps_affine: Mat4,
    pub identity: DicomVolumeIdentity,
    /// Window width and centre, in MR intensity units.
    pub window_level: Option<[f32; 2]>,
}

/// Incrementally decode an explicitly selected series without retaining input bytes.
#[derive(Default)]
pub struct MrSeriesBuilder {
    grid: Option<Grid>,
    slices: Vec<Slice>,
    uids: BTreeSet<String>,
    bytes: usize,
    voxels: usize,
}

impl MrSeriesBuilder {
    /// Decode one Part 10 instance. A failed append leaves the builder unchanged.
    pub fn push(&mut self, bytes: &[u8]) -> KeplerResult<()> {
        let total = self
            .bytes
            .checked_add(bytes.len())
            .filter(|&n| {
                n <= MAX_BYTES && bytes.len() <= 256 * 1024 * 1024 && self.slices.len() < MAX_SLICES
            })
            .ok_or_else(|| invalid("input budget exceeded"))?;
        let prefix = bytes
            .get(128..bytes.len().min(1024 * 1024))
            .ok_or_else(|| invalid("missing DICOM file meta"))?;
        let meta =
            FileMetaTable::from_reader(prefix).map_err(|_| invalid("invalid DICOM file meta"))?;
        if meta.media_storage_sop_class_uid() != MR_IMAGE_STORAGE {
            return Err(invalid("expected classic MR Image Storage"));
        }
        if !matches!(
            meta.transfer_syntax(),
            "1.2.840.10008.1.2" | "1.2.840.10008.1.2.1"
        ) {
            return Err(invalid(
                "requires uncompressed little-endian transfer syntax",
            ));
        }
        let o = FileDicomObject::from_reader(bytes)
            .map_err(|_| invalid("cannot decode DICOM instance"))?;
        if text(&o, "SOPClassUID").as_deref() != Some(MR_IMAGE_STORAGE) {
            return Err(invalid("dataset is not classic MR Image Storage"));
        }
        if o.element_by_name("NumberOfFrames").is_ok()
            && number::<usize>(&o, "NumberOfFrames") != Some(1)
        {
            return Err(invalid("requires one frame per instance"));
        }
        let stored = number::<u32>(&o, "BitsStored").filter(|n| (1..=16).contains(n));
        let representation = number::<u32>(&o, "PixelRepresentation");
        if text(&o, "PhotometricInterpretation").as_deref() != Some("MONOCHROME2")
            || number::<u32>(&o, "SamplesPerPixel") != Some(1)
            || number::<u32>(&o, "BitsAllocated") != Some(16)
            || stored.is_none()
            || number::<u32>(&o, "HighBit") != stored.map(|n| n - 1)
            || !matches!(representation, Some(0 | 1))
        {
            return Err(invalid("unsupported grayscale pixel layout"));
        }
        for (key, expected) in [("RescaleSlope", 1.0), ("RescaleIntercept", 0.0)] {
            if o.element_by_name(key).is_ok() && number::<f64>(&o, key) != Some(expected) {
                return Err(invalid("nonidentity intensity rescale is unsupported"));
            }
        }
        if o.element_by_name("ModalityLUTSequence").is_ok()
            || o.element_by_name("RealWorldValueMappingSequence").is_ok()
        {
            return Err(invalid("mapped MR intensities are unsupported"));
        }
        let dimension = |key| {
            number::<usize>(&o, key)
                .filter(|&n| n > 0)
                .ok_or_else(|| invalid("missing or invalid image dimensions"))
        };
        let [row_spacing, column_spacing] = values(&o, "PixelSpacing")?;
        let grid = Grid {
            dimensions: [dimension("Columns")?, dimension("Rows")?],
            spacing: [column_spacing, row_spacing],
            series: uid(&o, "SeriesInstanceUID")?,
            frame: uid(&o, "FrameOfReferenceUID")?,
            acquisition: ["EchoNumbers", "EchoTime", "TemporalPositionIdentifier"]
                .map(|key| text(&o, key)),
        };
        if grid.spacing.iter().any(|&n| n <= 0.0)
            || self.grid.as_ref().is_some_and(|old| old != &grid)
        {
            return Err(invalid(
                "inconsistent series, acquisition, frame, dimensions or pixel spacing",
            ));
        }
        let count = grid.dimensions[0]
            .checked_mul(grid.dimensions[1])
            .filter(|&n| n <= MAX_VOXELS - self.voxels)
            .ok_or_else(|| invalid("voxel budget exceeded"))?;
        let id = uid(&o, "SOPInstanceUID")?;
        if id != meta.media_storage_sop_instance_uid() {
            return Err(invalid("file meta and dataset instance UIDs differ"));
        }
        if self.uids.contains(&id) {
            return Err(invalid("duplicate SOP Instance UID"));
        }
        let [a, b, c, d, e, f] = values(&o, "ImageOrientationPatient")?;
        let (x, y) = (DVec3::new(a, b, c), DVec3::new(d, e, f));
        if (x.length() - 1.0).abs() > ORIENTATION_TOLERANCE
            || (y.length() - 1.0).abs() > ORIENTATION_TOLERANCE
            || x.dot(y).abs() > ORIENTATION_TOLERANCE
        {
            return Err(invalid("invalid image orientation"));
        }
        let origin = DVec3::from_array(values(&o, "ImagePositionPatient")?);
        let stored = stored.ok_or_else(|| invalid("missing stored bit count"))?;
        let element = o
            .element_by_name("PixelData")
            .map_err(|_| invalid("missing Pixel Data"))?;
        let words = element
            .to_multi_int::<u16>()
            .map_err(|_| invalid("invalid Pixel Data words"))?;
        if words.len() != count {
            return Err(invalid("pixel count does not match dimensions"));
        }
        let shift = 16 - stored;
        let pixels = words
            .into_iter()
            .map(|word| {
                let shifted = word << shift;
                if representation == Some(1) {
                    Ok(i16::from_ne_bytes(shifted.to_ne_bytes()) >> shift)
                } else {
                    i16::try_from(shifted >> shift)
                        .map_err(|_| invalid("unsigned sample exceeds signed storage range"))
                }
            })
            .collect::<KeplerResult<_>>()?;
        let first_value = |key| {
            text(&o, key)?
                .split('\\')
                .next()?
                .parse::<f32>()
                .ok()
                .filter(|v| v.is_finite())
        };
        let window_level = first_value("WindowWidth")
            .filter(|v| *v >= 1.0)
            .zip(first_value("WindowCenter"))
            .map(|(w, l)| [w, l]);
        let single_spacing = number::<f64>(&o, "SpacingBetweenSlices")
            .or_else(|| number::<f64>(&o, "SliceThickness"))
            .filter(|n| n.is_finite() && *n > 0.0);
        self.grid = Some(grid);
        self.uids.insert(id.clone());
        self.slices.push(Slice {
            uid: id,
            origin,
            x,
            y,
            single_spacing,
            window_level,
            pixels,
        });
        self.bytes = total;
        self.voxels += count;
        Ok(())
    }

    /// Sort by patient position and require a regular orthogonal stack.
    pub fn finish(mut self, expected_series: &str) -> KeplerResult<ImportedMrSeries> {
        let grid = self.grid.ok_or_else(|| invalid("empty series"))?;
        if grid.series != expected_series {
            return Err(invalid("series changed since inspection"));
        }
        let first = &self.slices[0];
        let (x, y) = (first.x.normalize(), first.y.normalize());
        let normal = x.cross(y).normalize();
        if self.slices.iter().any(|s| {
            !s.x.abs_diff_eq(x, ORIENTATION_TOLERANCE) || !s.y.abs_diff_eq(y, ORIENTATION_TOLERANCE)
        }) {
            return Err(invalid("mixed slice orientations"));
        }
        self.slices
            .sort_by(|a, b| a.origin.dot(normal).total_cmp(&b.origin.dot(normal)));
        let first = &self.slices[0];
        let origin = first.origin;
        let depth = self.slices.len();
        let z = if depth > 1 {
            (self.slices[depth - 1].origin - origin).dot(normal) / (depth - 1) as f64
        } else {
            first
                .single_spacing
                .ok_or_else(|| invalid("single slice requires spacing or thickness"))?
        };
        let step = normal * z;
        if z <= POSITION_TOLERANCE_MM
            || self
                .slices
                .windows(2)
                .any(|s| (s[1].origin - s[0].origin).dot(normal) <= POSITION_TOLERANCE_MM)
            || self.slices.iter().enumerate().any(|(i, s)| {
                (s.origin - origin - step * i as f64).length() > POSITION_TOLERANCE_MM
            })
        {
            return Err(invalid("duplicate, irregular or shifted slice planes"));
        }
        let window_level = self.slices[depth / 2].window_level;
        let [sx, sy] = grid.spacing;
        let affine = Mat4::from_cols(
            (x * sx).as_vec3().extend(0.0),
            (y * sy).as_vec3().extend(0.0),
            step.as_vec3().extend(0.0),
            origin.as_vec3().extend(1.0),
        );
        if !affine.is_finite() || !affine.inverse().is_finite() {
            return Err(invalid("invalid patient transform"));
        }
        let [width, height] = grid.dimensions;
        let image_uids = self.slices.iter().map(|s| s.uid.clone()).collect();
        let mut pixels = Vec::with_capacity(self.voxels);
        for mut slice in self.slices {
            pixels.append(&mut slice.pixels);
        }
        Ok(ImportedMrSeries {
            volume: CTVolume::new(
                (width, height, depth),
                (sx as f32, sy as f32, z as f32),
                pixels,
                Base::default(),
            ),
            patient_lps_affine: affine,
            identity: DicomVolumeIdentity {
                frame_of_reference_uid: grid.frame,
                series_instance_uid: grid.series,
                image_uids,
            },
            window_level,
        })
    }
}
