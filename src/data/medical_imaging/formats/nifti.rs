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
#[cfg(test)]
mod tests {
    use super::*;

    /// Builds a minimal 348-byte NIfTI-1 header with the given byte order.
    /// Layout offsets per the NIfTI-1 spec: dim[0]@40, datatype@70,
    /// pixdim[1..3]@76+4i ( pixdim[1]=x spacing @80 ), vox_offset@108,
    /// sform_code@254, srow_x/y/z @280/296/312.
    fn header(le: bool, datatype: i16) -> Vec<u8> {
        let mut b = vec![0u8; 348];
        let put_i32 = |b: &mut Vec<u8>, o: usize, v: i32| {
            let arr = if le { v.to_le_bytes() } else { v.to_be_bytes() };
            b[o..o + 4].copy_from_slice(&arr);
        };
        let put_i16 = |b: &mut Vec<u8>, o: usize, v: i16| {
            let arr = if le { v.to_le_bytes() } else { v.to_be_bytes() };
            b[o..o + 2].copy_from_slice(&arr);
        };
        let put_f32 = |b: &mut Vec<u8>, o: usize, v: f32| {
            let arr = if le { v.to_le_bytes() } else { v.to_be_bytes() };
            b[o..o + 4].copy_from_slice(&arr);
        };
        put_i32(&mut b, 0, NIFTI1_HEADER_SIZE as i32); // sizeof_hdr
        put_i16(&mut b, 40, 3); // dim[0] = 3 dimensions
        put_i16(&mut b, 42, 64); // dim[1] x
        put_i16(&mut b, 44, 64); // dim[2] y
        put_i16(&mut b, 46, 32); // dim[3] z
        put_i16(&mut b, 70, datatype);
        put_f32(&mut b, 80, 0.5); // pixdim[1] x spacing
        put_f32(&mut b, 84, 0.5); // pixdim[2] y spacing
        put_f32(&mut b, 88, 1.0); // pixdim[3] z spacing
        put_f32(&mut b, 108, NIFTI1_HEADER_SIZE as f32 + 4.0); // vox_offset = 352
        put_i16(&mut b, 254, 1); // sform_code = scanner anat
        // srow: identity orientation + origin (10, 20, 30)
        put_f32(&mut b, 280, 1.0);
        put_f32(&mut b, 284, 0.0);
        put_f32(&mut b, 288, 0.0);
        put_f32(&mut b, 292, 10.0);
        put_f32(&mut b, 296, 0.0);
        put_f32(&mut b, 300, 1.0);
        put_f32(&mut b, 304, 0.0);
        put_f32(&mut b, 308, 20.0);
        put_f32(&mut b, 312, 0.0);
        put_f32(&mut b, 316, 0.0);
        put_f32(&mut b, 320, 1.0);
        put_f32(&mut b, 324, 30.0);
        b
    }

    #[test]
    fn little_endian_header_parses_all_fields() {
        let meta = NiftiParser::parse_metadata_only(&header(true, 4)).unwrap();
        assert_eq!(meta.dimensions, vec![64, 64, 32]);
        assert_eq!(meta.pixel_type, PixelType::Int16);
        assert_eq!(meta.spacing, vec![0.5, 0.5, 1.0]);
        assert_eq!(meta.offset, vec![10.0, 20.0, 30.0]);
        assert_eq!(meta.endianness, Endianness::Little);
        assert_eq!(meta.data_offset, Some(352));
        assert_eq!(meta.total_pixels(), 64 * 64 * 32);
    }

    #[test]
    fn big_endian_header_is_detected_and_parsed() {
        let meta = NiftiParser::parse_metadata_only(&header(false, 2)).unwrap();
        assert_eq!(meta.endianness, Endianness::Big);
        assert_eq!(meta.dimensions, vec![64, 64, 32]);
        assert_eq!(meta.pixel_type, PixelType::UInt8);
        assert_eq!(meta.offset, vec![10.0, 20.0, 30.0]);
    }

    #[test]
    fn datatype_codes_map_to_pixel_types() {
        let cases = [
            (2i16, PixelType::UInt8),
            (256, PixelType::UInt8),
            (4, PixelType::Int16),
            (512, PixelType::UInt16),
            (8, PixelType::Int32),
            (16, PixelType::Float32),
            (64, PixelType::Float64),
        ];
        for (code, expected) in cases {
            let meta = NiftiParser::parse_metadata_only(&header(true, code)).unwrap();
            assert_eq!(meta.pixel_type, expected, "datatype {code}");
        }
    }

    #[test]
    fn unsupported_datatype_is_rejected() {
        let err = NiftiParser::parse_metadata_only(&header(true, 3)).unwrap_err();
        assert!(err.to_string().contains("datatype"), "got: {err}");
    }

    #[test]
    fn short_buffer_is_rejected() {
        let err = NiftiParser::parse_metadata_only(&vec![0u8; 100]).unwrap_err();
        assert!(err.to_string().contains("348"), "got: {err}");
    }

    #[test]
    fn bad_sizeof_hdr_is_rejected() {
        let mut b = header(true, 4);
        b[0..4].copy_from_slice(&999i32.to_le_bytes());
        let err = NiftiParser::parse_metadata_only(&b).unwrap_err();
        assert!(err.to_string().contains("sizeof_hdr"), "got: {err}");
    }

    #[test]
    fn qform_only_file_falls_back_to_identity_and_zero_offset() {
        let mut b = header(true, 4);
        b[254..256].copy_from_slice(&0i16.to_le_bytes()); // sform_code = 0
        let meta = NiftiParser::parse_metadata_only(&b).unwrap();
        assert_eq!(meta.offset, vec![0.0, 0.0, 0.0]);
        assert_eq!(
            meta.orientation,
            [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]]
        );
    }

    #[test]
    fn non_positive_spacing_falls_back_to_unit() {
        let mut b = header(true, 4);
        let z = 0.0f32.to_le_bytes();
        b[80..84].copy_from_slice(&z);
        b[84..88].copy_from_slice(&z);
        let meta = NiftiParser::parse_metadata_only(&b).unwrap();
        assert_eq!(meta.spacing, vec![1.0, 1.0, 1.0]);
    }

    #[test]
    fn non_positive_dim_falls_back_to_one() {
        let mut b = header(true, 4);
        let z = 0i16.to_le_bytes();
        b[42..44].copy_from_slice(&z); // dim[1] = 0
        let meta = NiftiParser::parse_metadata_only(&b).unwrap();
        assert_eq!(meta.dimensions[0], 1);
    }

    #[test]
    fn parse_bytes_extracts_voxels_after_vox_offset() {
        let mut b = header(true, 4);
        // Point vox_offset at the first byte after the 348-byte header.
        b[108..112].copy_from_slice(&348.0f32.to_le_bytes());
        b.extend_from_slice(&[1u8, 2, 3, 4, 5, 6]); // voxel payload at 348
        let vol = NiftiParser::parse_bytes(&b).unwrap();
        assert_eq!(vol.source_format, ImageFormat::NIfTI);
        match vol.pixel_data {
            PixelData::UInt8(v) => assert_eq!(v, vec![1, 2, 3, 4, 5, 6]),
            other => panic!("expected UInt8 payload, got {other:?}"),
        }
    }

    #[test]
    fn parse_bytes_with_vox_offset_beyond_end_is_rejected() {
        let mut b = header(true, 4);
        let huge = 1.0e9f32; // far beyond file end
        b[108..112].copy_from_slice(&huge.to_le_bytes());
        let err = NiftiParser::parse_bytes(&b).unwrap_err();
        assert!(err.to_string().contains("beyond"), "got: {err}");
    }

    #[test]
    fn gzip_wrapped_header_is_transparently_decompressed() {
        let raw = header(true, 16); // Float32
        let mut enc = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
        std::io::Write::write_all(&mut enc, &raw).unwrap();
        let gz = enc.finish().unwrap();
        let meta = NiftiParser::parse_metadata_only(&gz).unwrap();
        assert_eq!(meta.pixel_type, PixelType::Float32);
        assert_eq!(meta.dimensions, vec![64, 64, 32]);
    }
}
