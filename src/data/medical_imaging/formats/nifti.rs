//! NIfTI-1 import. Native paths and browser bytes share validation and conversion.
//!
//! CPU storage is canonical signed i16, independently of display window/level.
//! See `doc/fileio/nifti.md` for space selection, conversion and rejection policies.
use std::{borrow::Cow, io::Read};

use glam::{Mat3, Mat4, Quat, Vec3, Vec4};

use crate::{
    core::coord::Base,
    data::{
        ct_volume::CTVolume,
        medical_imaging::{
            error::{MedicalImagingError, MedicalImagingResult},
            metadata::{
                Endianness, ImageMetadata, MedicalVolume, PatientPosition, PixelData, PixelType,
            },
            ImageFormat,
        },
    },
};

/// Resource limits apply before allocation, also to gzip expansion.
#[derive(Clone, Copy, Debug)]
pub struct NiftiImportOptions {
    pub max_input_bytes: usize,
    pub max_decompressed_bytes: usize,
    pub max_voxel_bytes: usize,
}

impl Default for NiftiImportOptions {
    fn default() -> Self {
        Self {
            max_input_bytes: 512 * 1024 * 1024,
            max_decompressed_bytes: 512 * 1024 * 1024,
            max_voxel_bytes: 512 * 1024 * 1024,
        }
    }
}

/// Meaning of the selected world space; LPS axis conversion does not change this identity.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NiftiReferenceSpace {
    Unknown,
    Scanner,
    Aligned,
    Talairach,
    Mni152,
    Template,
    Other(i16),
}

impl NiftiReferenceSpace {
    fn from_code(code: i16) -> Self {
        match code {
            0 => Self::Unknown,
            1 => Self::Scanner,
            2 => Self::Aligned,
            3 => Self::Talairach,
            4 => Self::Mni152,
            5 => Self::Template,
            code => Self::Other(code),
        }
    }
}

/// Original NIfTI transforms in their declared RAS units, retained for provenance.
#[derive(Clone, Debug)]
pub struct NiftiTransforms {
    pub qform_code: i16,
    pub sform_code: i16,
    pub qform: Option<Mat4>,
    pub sform: Option<Mat4>,
}

/// Resolved geometry. `CTVolume::base()` is the authoritative voxel-centre affine.
#[derive(Clone, Debug)]
pub struct NiftiGeometry {
    pub reference_space: NiftiReferenceSpace,
    pub units_known: bool,
    pub world_to_voxel: Mat4,
    pub transforms: NiftiTransforms,
}

/// Import diagnostics describe lossy conversion without changing display parameters.
#[derive(Clone, Debug, Default)]
pub struct NiftiDiagnostics {
    pub clipped_voxels: usize,
    pub rounded_voxels: usize,
    pub warnings: Vec<String>,
}

/// Fully validated, converted volume, ready for the existing GPU upload path.
#[derive(Debug)]
pub struct ImportedNifti {
    pub volume: CTVolume,
    pub geometry: NiftiGeometry,
    pub is_2d: bool,
    pub diagnostics: NiftiDiagnostics,
}

impl ImportedNifti {
    /// Validated scanner LPS geometry in millimetres. Atlas/aligned/unknown spaces
    /// must not acquire scanner coordinates merely because their axes were converted.
    pub fn patient_lps_affine(&self) -> Option<Mat4> {
        (self.geometry.units_known && self.geometry.reference_space == NiftiReferenceSpace::Scanner)
            .then(|| self.volume.base().matrix)
    }
}

impl NiftiGeometry {
    pub fn space_label(&self) -> &'static str {
        match self.reference_space {
            NiftiReferenceSpace::Unknown => "orientation unknown",
            NiftiReferenceSpace::Scanner => "scanner LPS",
            NiftiReferenceSpace::Aligned => "aligned LPS (not scanner space)",
            NiftiReferenceSpace::Talairach => "Talairach LPS (not scanner space)",
            NiftiReferenceSpace::Mni152 => "MNI152 LPS (not scanner space)",
            NiftiReferenceSpace::Template => "template LPS (not scanner space)",
            NiftiReferenceSpace::Other(_) => "unrecognized reference space",
        }
    }
}

#[derive(Debug)]
struct Header {
    le: bool,
    dimensions: [usize; 3],
    rank: usize,
    datatype: i16,
    bytes_per_voxel: usize,
    pixel_type: PixelType,
    offset: usize,
    slope: f64,
    intercept: f64,
    affine: Mat4,
    geometry: NiftiGeometry,
    warnings: Vec<String>,
}

fn invalid(field: &str, reason: impl Into<String>) -> MedicalImagingError {
    MedicalImagingError::ParseError {
        field: format!("NIfTI {field}"),
        reason: reason.into(),
    }
}

fn limit(size: usize, max: usize, field: &str) -> MedicalImagingResult<()> {
    if size > max {
        return Err(invalid(field, format!("{size} bytes exceed limit {max}")));
    }
    Ok(())
}

fn unpack(bytes: &[u8], options: NiftiImportOptions) -> MedicalImagingResult<Cow<'_, [u8]>> {
    limit(bytes.len(), options.max_input_bytes, "input")?;
    if !bytes.starts_with(&[0x1f, 0x8b]) {
        limit(
            bytes.len(),
            options.max_decompressed_bytes,
            "expanded input",
        )?;
        return Ok(Cow::Borrowed(bytes));
    }
    let mut decoder = flate2::read::MultiGzDecoder::new(bytes);
    let mut expanded = Vec::new();
    let mut scratch = [0; 16 * 1024];
    loop {
        let n = decoder
            .read(&mut scratch)
            .map_err(|e| invalid("gzip", e.to_string()))?;
        if n == 0 {
            break;
        }
        let size = expanded
            .len()
            .checked_add(n)
            .ok_or_else(|| invalid("gzip", "size overflow"))?;
        limit(size, options.max_decompressed_bytes, "expanded input")?;
        if size > expanded.capacity() {
            let capacity = expanded
                .capacity()
                .saturating_mul(2)
                .max(size)
                .min(options.max_decompressed_bytes);
            expanded
                .try_reserve_exact(capacity - expanded.len())
                .map_err(|_| MedicalImagingError::MemoryAllocation { size: capacity })?;
        }
        expanded.extend_from_slice(&scratch[..n]);
    }
    Ok(Cow::Owned(expanded))
}

fn checked_inverse(matrix: Mat4) -> MedicalImagingResult<Mat4> {
    let determinant = matrix.determinant();
    if !matrix.is_finite() || !determinant.is_finite() || determinant == 0.0 {
        return Err(invalid("transform", "non-finite or singular affine"));
    }
    let inverse = matrix.inverse();
    if !inverse.is_finite() {
        return Err(invalid("transform", "inverse is not finite"));
    }
    Ok(inverse)
}

impl Header {
    fn parse(raw: &[u8]) -> MedicalImagingResult<Self> {
        if raw.len() < 348 {
            return Err(invalid("header", "requires 348 bytes"));
        }
        let le = raw[..4] == 348_i32.to_le_bytes();
        if !le && raw[..4] != 348_i32.to_be_bytes() {
            return Err(invalid("sizeof_hdr", "expected NIfTI-1 header size 348"));
        }
        if &raw[344..348] != b"n+1\0" {
            return Err(invalid(
                "magic",
                "only single-file NIfTI-1 n+1 is supported",
            ));
        }
        let short = |o| {
            let b = [raw[o], raw[o + 1]];
            if le {
                i16::from_le_bytes(b)
            } else {
                i16::from_be_bytes(b)
            }
        };
        let float = |o| {
            let b = [raw[o], raw[o + 1], raw[o + 2], raw[o + 3]];
            if le {
                f32::from_le_bytes(b)
            } else {
                f32::from_be_bytes(b)
            }
        };
        let rank = short(40);
        if !(2..=7).contains(&rank) {
            return Err(invalid(
                "dim[0]",
                "expected spatial scalar image with rank 2..7",
            ));
        }
        let mut dimensions = [1; 3];
        for axis in 1..=rank as usize {
            let length = short(40 + axis * 2);
            if length <= 0 {
                return Err(invalid("dimensions", "active dimensions must be positive"));
            }
            if axis <= 3 {
                dimensions[axis - 1] = length as usize;
            } else if length != 1 {
                return Err(invalid(
                    "dimensions",
                    "non-singleton time/vector dimensions are unsupported",
                ));
            }
        }
        let datatype = short(70);
        let (bytes_per_voxel, pixel_type) = match datatype {
            2 => (1, PixelType::UInt8),
            // The legacy PixelType API has no Int8 variant. The importer decodes signed bytes
            // directly; legacy parse_bytes expands this type to Int16 below.
            256 => (1, PixelType::Int16),
            4 => (2, PixelType::Int16),
            512 => (2, PixelType::UInt16),
            8 => (4, PixelType::Int32),
            16 => (4, PixelType::Float32),
            64 => (8, PixelType::Float64),
            _ => {
                return Err(MedicalImagingError::UnsupportedPixelType {
                    pixel_type: format!("NIfTI datatype {datatype}"),
                })
            }
        };
        if short(72) != bytes_per_voxel * 8 {
            return Err(invalid("bitpix", "does not match datatype"));
        }
        let offset = float(108);
        if !offset.is_finite() || f64::from(offset) >= usize::MAX as f64 {
            return Err(invalid(
                "vox_offset",
                "not a finite representable byte offset",
            ));
        }
        // NIfTI defines (int)vox_offset and normalizes single-file offsets below 352.
        let offset = offset.max(352.0) as usize;
        let slope = f64::from(float(112));
        let (slope, intercept) = if slope == 0.0 {
            (1.0, 0.0)
        } else {
            (slope, f64::from(float(116)))
        };
        if !slope.is_finite() || !intercept.is_finite() {
            return Err(invalid("scaling", "active slope/intercept must be finite"));
        }
        let units = raw[123] & 7;
        let unit_scale = match units {
            0 | 2 => 1.0,
            1 => 1000.0,
            3 => 0.001,
            _ => return Err(invalid("units", "reserved spatial unit code")),
        };
        let mut warnings = Vec::new();
        if units == 0 {
            warnings.push("Spatial units unknown; distances are not millimetres".into());
        }
        let qform_code = short(252);
        let sform_code = short(254);
        if qform_code < 0 || sform_code < 0 {
            return Err(invalid("transform codes", "negative code"));
        }
        let spacing = || -> MedicalImagingResult<Vec3> {
            let s = Vec3::new(
                float(80),
                float(84),
                if rank == 2 { 1.0 } else { float(88) },
            );
            if !s.is_finite() || s.min_element() <= 0.0 {
                return Err(invalid(
                    "pixdim",
                    "spatial spacing must be positive and finite",
                ));
            }
            Ok(s)
        };
        let qform = if qform_code > 0 {
            let mut bcd = Vec3::new(float(256), float(260), float(264));
            let norm = bcd.length_squared();
            if !norm.is_finite() {
                return Err(invalid("quaternion", "non-finite quaternion"));
            }
            let a = if 1.0 - norm < 1e-7 {
                bcd = bcd.normalize();
                0.0
            } else {
                (1.0 - norm).sqrt()
            };
            let qfac = float(76);
            if !qfac.is_finite() {
                return Err(invalid("qfac", "non-finite qfac"));
            }
            let scale = spacing()? * Vec3::new(1.0, 1.0, if qfac < 0.0 { -1.0 } else { 1.0 });
            let translation = Vec3::new(float(268), float(272), float(276));
            let m = Mat4::from_scale_rotation_translation(
                scale,
                Quat::from_xyzw(bcd.x, bcd.y, bcd.z, a),
                translation,
            );
            checked_inverse(m)?;
            Some(m)
        } else {
            None
        };
        let sform = if sform_code > 0 {
            let m = Mat4::from_cols(
                Vec4::new(float(280), float(296), float(312), 0.0),
                Vec4::new(float(284), float(300), float(316), 0.0),
                Vec4::new(float(288), float(304), float(320), 0.0),
                Vec4::new(float(292), float(308), float(324), 1.0),
            );
            checked_inverse(m)?;
            Some(m)
        } else {
            None
        };
        if qform_code == 1 && sform_code == 1 {
            if let (Some(q), Some(s)) = (qform, sform) {
                // Compare voxel centres/corners, not just matrix coefficients.
                let max = Vec3::from_array(dimensions.map(|n| (n - 1) as f32));
                for mask in 0..8 {
                    let p = Vec3::new(
                        if mask & 1 == 0 { 0.0 } else { max.x },
                        if mask & 2 == 0 { 0.0 } else { max.y },
                        if mask & 4 == 0 { 0.0 } else { max.z },
                    );
                    if (q.transform_point3(p) - s.transform_point3(p)).length() * unit_scale > 0.01
                    {
                        return Err(invalid("transforms", "conflicting scanner qform/sform (over 0.01 spatial units, mm when known)"));
                    }
                }
            }
        }
        let (selected, code) = if qform_code == 1 {
            (qform, qform_code)
        } else if sform.is_some() {
            (sform, sform_code)
        } else {
            (qform, qform_code)
        };
        // Unknown orientation keeps a spacing-only voxel frame, not guessed RAS/LPS axes.
        let affine = if let Some(m) = selected {
            Mat4::from_scale(Vec3::new(-unit_scale, -unit_scale, unit_scale)) * m
        } else {
            Mat4::from_scale(spacing()? * unit_scale)
        };
        let world_to_voxel = checked_inverse(affine)?;
        for axis in [affine.x_axis, affine.y_axis, affine.z_axis] {
            let length = axis.truncate().length();
            if !length.is_finite() || length <= 0.0 {
                return Err(invalid(
                    "transform",
                    "voxel step cannot be represented as finite spacing",
                ));
            }
        }
        let geometry = NiftiGeometry {
            reference_space: NiftiReferenceSpace::from_code(code),
            units_known: units != 0,
            world_to_voxel,
            transforms: NiftiTransforms {
                qform_code,
                sform_code,
                qform,
                sform,
            },
        };
        Ok(Self {
            le,
            dimensions,
            rank: rank as usize,
            datatype,
            bytes_per_voxel: bytes_per_voxel as usize,
            pixel_type,
            offset,
            slope,
            intercept,
            affine,
            geometry,
            warnings,
        })
    }

    fn payload<'a>(&self, raw: &'a [u8]) -> MedicalImagingResult<&'a [u8]> {
        let count = self
            .dimensions
            .iter()
            .try_fold(1_usize, |n, &d| n.checked_mul(d))
            .ok_or_else(|| invalid("dimensions", "voxel count overflow"))?;
        let bytes = count
            .checked_mul(self.bytes_per_voxel)
            .ok_or_else(|| invalid("dimensions", "byte count overflow"))?;
        let end = self
            .offset
            .checked_add(bytes)
            .ok_or_else(|| invalid("vox_offset", "payload end overflow"))?;
        raw.get(self.offset..end)
            .ok_or(MedicalImagingError::DataCorruption {
                expected: end,
                actual: raw.len(),
            })
    }

    fn metadata(&self) -> ImageMetadata {
        let columns = [
            self.affine.x_axis.truncate(),
            self.affine.y_axis.truncate(),
            self.affine.z_axis.truncate(),
        ];
        let spacing = columns.map(Vec3::length);
        let orientation = Mat3::from_cols(
            columns[0] / spacing[0],
            columns[1] / spacing[1],
            columns[2] / spacing[2],
        )
        .transpose()
        .to_cols_array_2d();
        ImageMetadata {
            dimensions: self.dimensions.to_vec(),
            spacing: spacing.to_vec(),
            offset: self.affine.w_axis.truncate().to_array().to_vec(),
            orientation,
            pixel_type: self.pixel_type,
            endianness: if self.le {
                Endianness::Little
            } else {
                Endianness::Big
            },
            compression: None,
            patient_position: PatientPosition::Unknown,
            data_offset: Some(self.offset),
            element_data_file: "NIFTI".into(),
        }
    }
}

/// Import a single-file NIfTI-1 image. Invalid input never constructs a volume.
pub fn import_nifti_bytes(
    bytes: &[u8],
    options: NiftiImportOptions,
) -> MedicalImagingResult<ImportedNifti> {
    let raw = unpack(bytes, options)?;
    let header = Header::parse(&raw)?;
    let payload = header.payload(&raw)?;
    let count = payload.len() / header.bytes_per_voxel;
    let output_size = count
        .checked_mul(2)
        .ok_or_else(|| invalid("voxels", "size overflow"))?;
    limit(output_size, options.max_voxel_bytes, "converted voxels")?;
    let axes = [
        header.affine.x_axis.truncate().normalize(),
        header.affine.y_axis.truncate().normalize(),
        header.affine.z_axis.truncate().normalize(),
    ];
    if axes[0].dot(axes[1]).abs() > 1e-4
        || axes[0].dot(axes[2]).abs() > 1e-4
        || axes[1].dot(axes[2]).abs() > 1e-4
    {
        return Err(invalid(
            "geometry",
            "selected affine contains shear; rendering import requires orthogonal axes",
        ));
    }
    let mut voxels = Vec::new();
    voxels
        .try_reserve_exact(count)
        .map_err(|_| MedicalImagingError::MemoryAllocation { size: output_size })?;
    let mut diagnostics = NiftiDiagnostics {
        warnings: header.warnings.clone(),
        ..Default::default()
    };
    for (index, chunk) in payload.chunks_exact(header.bytes_per_voxel).enumerate() {
        let mut word = [0; 8];
        word[..chunk.len()].copy_from_slice(chunk);
        let value = match header.datatype {
            2 => f64::from(chunk[0]),
            256 => f64::from(chunk[0] as i8),
            4 => f64::from(if header.le {
                i16::from_le_bytes([word[0], word[1]])
            } else {
                i16::from_be_bytes([word[0], word[1]])
            }),
            512 => f64::from(if header.le {
                u16::from_le_bytes([word[0], word[1]])
            } else {
                u16::from_be_bytes([word[0], word[1]])
            }),
            8 => f64::from(if header.le {
                i32::from_le_bytes([word[0], word[1], word[2], word[3]])
            } else {
                i32::from_be_bytes([word[0], word[1], word[2], word[3]])
            }),
            16 => f64::from(if header.le {
                f32::from_le_bytes([word[0], word[1], word[2], word[3]])
            } else {
                f32::from_be_bytes([word[0], word[1], word[2], word[3]])
            }),
            64 => {
                if header.le {
                    f64::from_le_bytes(word)
                } else {
                    f64::from_be_bytes(word)
                }
            }
            _ => return Err(invalid("datatype", "unsupported scalar type")),
        };
        let scaled = value * header.slope + header.intercept;
        if !scaled.is_finite() {
            return Err(invalid(
                "voxels",
                format!("non-finite scaled value at index {index}"),
            ));
        }
        let rounded = scaled.round();
        diagnostics.rounded_voxels += usize::from(rounded != scaled);
        diagnostics.clipped_voxels +=
            usize::from(rounded < f64::from(i16::MIN) || rounded > f64::from(i16::MAX));
        voxels.push(rounded.clamp(f64::from(i16::MIN), f64::from(i16::MAX)) as i16);
    }
    let s = header.metadata().spacing;
    let [x, y, z] = header.dimensions;
    Ok(ImportedNifti {
        volume: CTVolume::new(
            (x, y, z),
            (s[0], s[1], s[2]),
            voxels,
            Base {
                label: "NIfTI voxel centres".into(),
                matrix: header.affine,
            },
        ),
        geometry: header.geometry,
        is_2d: header.rank == 2,
        diagnostics,
    })
}

/// Read a native file with the same limits and importer used by browser bytes.
#[cfg(not(target_arch = "wasm32"))]
pub fn open_nifti(
    path: &std::path::Path,
    options: NiftiImportOptions,
) -> MedicalImagingResult<ImportedNifti> {
    let file = std::fs::File::open(path)?;
    let metadata = file.metadata()?;
    if !metadata.is_file() {
        return Err(invalid("path", "expected regular file"));
    }
    if metadata.len() > options.max_input_bytes as u64 {
        return Err(invalid("input", "file exceeds input byte limit"));
    }
    let mut bytes = Vec::new();
    file.take((options.max_input_bytes as u64).saturating_add(1))
        .read_to_end(&mut bytes)?;
    import_nifti_bytes(&bytes, options)
}

/// Compatibility parser. New display callers should use `import_nifti_bytes`.
pub struct NiftiParser;

impl NiftiParser {
    pub fn parse_metadata_only(bytes: &[u8]) -> MedicalImagingResult<ImageMetadata> {
        let raw = unpack(bytes, NiftiImportOptions::default())?;
        Ok(Header::parse(&raw)?.metadata())
    }

    /// Validated raw payload, retaining the public MedicalVolume API without applying CT policy.
    pub fn parse_bytes(bytes: &[u8]) -> MedicalImagingResult<MedicalVolume> {
        let raw = unpack(bytes, NiftiImportOptions::default())?;
        let header = Header::parse(&raw)?;
        let payload = header.payload(&raw)?;
        let mut metadata = header.metadata();
        let data = if header.datatype == 256 {
            limit(
                payload
                    .len()
                    .checked_mul(2)
                    .ok_or_else(|| invalid("voxels", "size overflow"))?,
                NiftiImportOptions::default().max_voxel_bytes,
                "expanded int8 voxels",
            )?;
            metadata.endianness = Endianness::Little;
            PixelData::Int16(payload.iter().map(|&n| i16::from(n as i8)).collect())
        } else {
            PixelData::UInt8(payload.to_vec())
        };
        MedicalVolume::new(metadata, data, ImageFormat::NIfTI)
    }
}
