#![allow(dead_code)]

use crate::data::volume_encoding::VolumeEncoding;
use anyhow::*;

#[derive(Debug, Clone, Copy)]
struct ValidatedVolumeUpload {
    expected_bytes: usize,
    bytes_per_row: u32,
}

fn validate_volume_upload(
    data_len: usize,
    width: u32,
    height: u32,
    depth: u32,
    max_dimension: u32,
) -> Result<ValidatedVolumeUpload> {
    ensure!(
        width > 0 && height > 0 && depth > 0,
        "Volume texture dimensions must be nonzero; got {width}x{height}x{depth}"
    );
    ensure!(
        width <= max_dimension && height <= max_dimension && depth <= max_dimension,
        "Volume dimensions {width}x{height}x{depth} exceed device 3D texture limit {max_dimension}"
    );

    let expected_bytes = usize::try_from(width)
        .ok()
        .and_then(|width| width.checked_mul(height as usize))
        .and_then(|voxels| voxels.checked_mul(depth as usize))
        .and_then(|voxels| voxels.checked_mul(2))
        .ok_or_else(|| {
            anyhow!("Volume byte size overflow for dimensions {width}x{height}x{depth}")
        })?;
    let bytes_per_row = width
        .checked_mul(2)
        .ok_or_else(|| anyhow!("Volume row byte size overflow for width {width}"))?;
    ensure!(
        data_len == expected_bytes,
        "Volume buffer length {data_len} does not match expected two-byte voxel data length {expected_bytes} for dimensions {width}x{height}x{depth}"
    );

    Ok(ValidatedVolumeUpload {
        expected_bytes,
        bytes_per_row,
    })
}

#[derive(Debug, Clone, Copy)]
pub struct VolumeDecodeParameters {
    pub is_packed_flag: u32,
    pub bias: f32,
}

pub struct RenderContent {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub sampler: wgpu::Sampler,
    pub texture_format: wgpu::TextureFormat,
    pub volume_encoding: VolumeEncoding,
}

impl RenderContent {
    pub fn decode_parameters(&self) -> VolumeDecodeParameters {
        VolumeDecodeParameters {
            is_packed_flag: u32::from(self.volume_encoding == VolumeEncoding::PackedI16Rg8),
            bias: self.volume_encoding.decode_bias(),
        }
    }

    /// Uploads bytes using the texture format defined by the volume encoding.
    pub fn from_bytes(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        bytes: &[u8],
        label: &str,
        width: u32,
        height: u32,
        depth: u32,
        volume_encoding: VolumeEncoding,
    ) -> Result<Self> {
        let upload = validate_volume_upload(
            bytes.len(),
            width,
            height,
            depth,
            device.limits().max_texture_dimension_3d,
        )
        .with_context(|| {
            format!("Rejected {volume_encoding:?} volume upload ({width}x{height}x{depth})")
        })?;
        let texture_format = volume_encoding.texture_format();
        let size = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: depth,
        };
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D3,
            format: texture_format,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                aspect: wgpu::TextureAspect::All,
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
            },
            bytes,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(upload.bytes_per_row),
                rows_per_image: Some(height),
            },
            size,
        );

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::MipmapFilterMode::Nearest,
            ..Default::default()
        });
        Ok(Self {
            texture,
            view,
            sampler,
            texture_format,
            volume_encoding,
        })
    }

    // Read a 3D texture from bytes (native half-float path)
    pub fn from_bytes_r16f(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        bytes: &[u8],
        label: &str,
        width: u32,
        height: u32,
        depth: u32,
        volume_encoding: VolumeEncoding,
    ) -> Result<Self> {
        ensure!(
            volume_encoding == VolumeEncoding::R16Float,
            "Expected R16Float encoding"
        );
        Self::from_bytes(
            device,
            queue,
            bytes,
            label,
            width,
            height,
            depth,
            volume_encoding,
        )
    }

    // Function to read a 3D texture from a file at compile time
    pub fn from_file_at_compile_time(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        label: &str,
        width: u32,
        height: u32,
        depth: u32,
    ) -> Result<Self> {
        // Use the canonical signed fixture, not the legacy offset-packed file.
        let raw = include_bytes!("../../../image/combined_pixel_array.bin");
        let voxels: Vec<i16> = raw
            .as_chunks::<2>()
            .0
            .iter()
            .map(|bytes| i16::from_le_bytes([bytes[0], bytes[1]]))
            .collect();
        let volume_encoding = VolumeEncoding::PackedI16Rg8;
        let bytes = volume_encoding.encode_volume(&voxels);
        Self::from_bytes(
            device,
            queue,
            &bytes,
            label,
            width,
            height,
            depth,
            volume_encoding,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_supported_limit_and_produces_two_byte_rows() {
        let upload = validate_volume_upload(512, 256, 1, 1, 256).unwrap();
        assert_eq!(upload.expected_bytes, 512);
        assert_eq!(upload.bytes_per_row, 512);
    }

    #[test]
    fn accepts_dimensions_below_a_limit_smaller_than_1024() {
        let upload = validate_volume_upload(32 * 16 * 2, 32, 16, 1, 64).unwrap();
        assert_eq!(upload.expected_bytes, 1024);
    }

    #[test]
    fn rejects_each_zero_axis() {
        for dimensions in [(0, 1, 1), (1, 0, 1), (1, 1, 0)] {
            assert!(
                validate_volume_upload(2, dimensions.0, dimensions.1, dimensions.2, 16).is_err()
            );
        }
    }

    #[test]
    fn rejects_axes_larger_than_the_device_limit() {
        for dimensions in [(17, 1, 1), (1, 17, 1), (1, 1, 17)] {
            assert!(
                validate_volume_upload(34, dimensions.0, dimensions.1, dimensions.2, 16).is_err()
            );
        }
    }

    #[test]
    fn rejects_byte_count_overflow_and_mismatched_buffers() {
        assert!(validate_volume_upload(0, u32::MAX, u32::MAX, u32::MAX, u32::MAX).is_err());
        assert!(validate_volume_upload(7, 2, 2, 1, 8).is_err());
        assert!(validate_volume_upload(9, 2, 2, 1, 8).is_err());
    }
}
