//! NIfTI-1 (.nii / .nii.gz) format parser.
//! Hand-written NIfTI-1 header parsing (348-byte fixed layout) + gzip via flate2,
//! so it works on both native and wasm32 (no filesystem, no heavy dependencies).
use crate::data::medical_imaging::{
    error::{MedicalImagingError, MedicalImagingResult},
    metadata::{Endianness, ImageMetadata, MedicalVolume, PatientPosition, PixelData, PixelType},
    ImageFormat,
};
use std::io::Read;

const NIFTI1_HEADER_SIZE: usize = 348;
const NIFTI1_VOX_OFFSET_DEFAULT: usize = 352;

pub struct NiftiParser;

impl NiftiParser {
    /// Decompress if the buffer starts with the gzip magic (`1f 8b`); otherwise pass through.
    fn maybe_gunzip(bytes: &[u8]) -> MedicalImagingResult<Vec<u8>> {
        if bytes.len() >= 2 && bytes[0] == 0x1f && bytes[1] == 0x8b {
            let mut out = Vec::new();
            flate2::read::GzDecoder::new(bytes)
                .read_to_end(&mut out)
                .map_err(|e| MedicalImagingError::ParseError {
                    field: "gzip".to_string(),
                    reason: e.to_string(),
                })?;
            Ok(out)
        } else {
            Ok(bytes.to_vec())
        }
    }

    /// Parses metadata from a NIfTI buffer (gzip-tolerant).
    pub fn parse_metadata_only(bytes: &[u8]) -> MedicalImagingResult<ImageMetadata> {
        let raw = Self::maybe_gunzip(bytes)?;
        Self::parse_metadata_raw(&raw)
    }

    fn parse_metadata_raw(raw: &[u8]) -> MedicalImagingResult<ImageMetadata> {
        if raw.len() < NIFTI1_HEADER_SIZE {
            return Err(MedicalImagingError::ParseError {
                field: "NIfTI".to_string(),
                reason: format!("file is {} bytes, smaller than the 348-byte NIfTI-1 header", raw.len()),
            });
        }

        // Endianness: sizeof_hdr == 348 in the file's own byte order.
        let le = i32::from_le_bytes([raw[0], raw[1], raw[2], raw[3]]) == NIFTI1_HEADER_SIZE as i32;
        if !le && i32::from_be_bytes([raw[0], raw[1], raw[2], raw[3]]) != NIFTI1_HEADER_SIZE as i32 {
            return Err(MedicalImagingError::ParseError {
                field: "NIfTI".to_string(),
                reason: "sizeof_hdr is neither 348 (LE) nor 348 (BE)".to_string(),
            });
        }
        let i16_at = |o: usize| -> i16 {
            if le { i16::from_le_bytes([raw[o], raw[o + 1]]) } else { i16::from_be_bytes([raw[o], raw[o + 1]]) }
        };
        let f32_at = |o: usize| -> f32 {
            let b = [raw[o], raw[o + 1], raw[o + 2], raw[o + 3]];
            if le { f32::from_le_bytes(b) } else { f32::from_be_bytes(b) }
        };

        let mut dimensions = Vec::with_capacity(3);
        for i in 1..=3 {
            let d = i16_at(40 + i * 2); // dim[1..3], dim[0] at offset 40
            dimensions.push(if d > 0 { d as usize } else { 1 });
        }
        let datatype = i16_at(70);

        let mut spacing = Vec::with_capacity(3);
        for i in 1..=3 {
            let p = f32_at(76 + i * 4); // pixdim[1..3] at offset 76
            spacing.push(if p > 0.0 { p } else { 1.0 });
        }

        let vox = f32_at(108); // vox_offset
        let vox_offset = if vox >= NIFTI1_HEADER_SIZE as f32 { vox as usize } else { NIFTI1_VOX_OFFSET_DEFAULT };

        // sform_code at 254; srow_x/y/z at 280/296/312. qform-only files fall back to identity.
        let sform_code = i16_at(254);
        let (offset, orientation) = if sform_code > 0 {
            let sx = [f32_at(280), f32_at(284), f32_at(288), f32_at(292)];
            let sy = [f32_at(296), f32_at(300), f32_at(304), f32_at(308)];
            let sz = [f32_at(312), f32_at(316), f32_at(320), f32_at(324)];
            (
                vec![sx[3], sy[3], sz[3]],
                [[sx[0], sx[1], sx[2]], [sy[0], sy[1], sy[2]], [sz[0], sz[1], sz[2]]],
            )
        } else {
            (vec![0.0, 0.0, 0.0], [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]])
        };

        let pixel_type = match datatype {
            2 | 256 => PixelType::UInt8,
            4 => PixelType::Int16,
            512 => PixelType::UInt16,
            8 => PixelType::Int32,
            16 => PixelType::Float32,
            64 => PixelType::Float64,
            other => {
                return Err(MedicalImagingError::UnsupportedFormat {
                    format: format!("NIfTI datatype {}", other),
                })
            }
        };

        Ok(ImageMetadata {
            dimensions,
            spacing,
            offset,
            orientation,
            pixel_type,
            endianness: if le { Endianness::Little } else { Endianness::Big },
            compression: None,
            patient_position: PatientPosition::HFS,
            data_offset: Some(vox_offset),
            element_data_file: "NIFTI".to_string(),
        })
    }

    /// Parses a whole NIfTI object (possibly gzip-compressed) into a `MedicalVolume`.
    /// Voxels are returned as raw bytes (`PixelData::UInt8`) so callers can reuse the
    /// same `generate_ct_volume_mha` path as MHA/MHD.
    pub fn parse_bytes(bytes: &[u8]) -> MedicalImagingResult<MedicalVolume> {
        let raw = Self::maybe_gunzip(bytes)?;
        let metadata = Self::parse_metadata_raw(&raw)?;
        let start = metadata.data_offset.unwrap_or(NIFTI1_VOX_OFFSET_DEFAULT);
        if start >= raw.len() {
            return Err(MedicalImagingError::ParseError {
                field: "NIfTI".to_string(),
                reason: format!("voxel offset {} beyond file end {}", start, raw.len()),
            });
        }
        let voxels = &raw[start..];
        let pixel_data = PixelData::UInt8(voxels.to_vec());
        MedicalVolume::new(metadata, pixel_data, ImageFormat::NIfTI)
    }
}