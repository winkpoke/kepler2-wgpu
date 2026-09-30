//! GPU storage encodings for canonical signed 16-bit voxels.

/// Texture representation, independent of the medical meaning of each voxel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VolumeEncoding {
    /// Exact little-endian offset-binary i16 stored in normalized R/G channels.
    PackedI16Rg8,
    /// Half-float storage; large integer values may be rounded.
    R16Float,
}

const I16_STORAGE_OFFSET: i32 = 32768;

/// Packs the entire i16 domain into RG8, with R as the low byte.
///
/// Offset-binary (rather than two's complement) keeps decoding affine, so
/// decoding linearly filtered channels also interpolates signed voxel values.
pub fn pack_i16_rg8(value: i16) -> [u8; 2] {
    ((i32::from(value) + I16_STORAGE_OFFSET) as u16).to_le_bytes()
}

/// Recovers an exact signed voxel from an offset-binary RG8 texel.
pub fn unpack_rg8_i16(bytes: [u8; 2]) -> i16 {
    (i32::from(u16::from_le_bytes(bytes)) - I16_STORAGE_OFFSET) as i16
}

impl VolumeEncoding {
    /// Chooses the preferred half-float path when filtered sampling is supported.
    pub fn for_float_support(use_float: bool) -> Self {
        if use_float {
            Self::R16Float
        } else {
            Self::PackedI16Rg8
        }
    }

    /// Encodes voxel data with explicit little-endian byte ordering.
    pub fn encode_volume(self, voxels: &[i16]) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(voxels.len() * 2);
        for &value in voxels {
            let encoded = match self {
                Self::PackedI16Rg8 => pack_i16_rg8(value),
                Self::R16Float => half::f16::from_f32(f32::from(value))
                    .to_bits()
                    .to_le_bytes(),
            };
            bytes.extend_from_slice(&encoded);
        }
        bytes
    }

    /// Returns the GPU format corresponding to the encoded bytes.
    pub fn texture_format(self) -> wgpu::TextureFormat {
        match self {
            Self::PackedI16Rg8 => wgpu::TextureFormat::Rg8Unorm,
            Self::R16Float => wgpu::TextureFormat::R16Float,
        }
    }

    /// Offset used by the shared WGSL decoder, not by window/level display logic.
    pub fn decode_bias(self) -> f32 {
        match self {
            Self::PackedI16Rg8 => I16_STORAGE_OFFSET as f32,
            Self::R16Float => 0.0,
        }
    }
}
