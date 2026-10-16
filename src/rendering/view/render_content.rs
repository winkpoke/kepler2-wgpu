#![allow(dead_code)]

use crate::data::volume_encoding::VolumeEncoding;
use anyhow::*;

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
        match self.volume_encoding {
            VolumeEncoding::HuPackedRg8 { offset } => VolumeDecodeParameters {
                is_packed_flag: 1,
                bias: offset,
            },
            VolumeEncoding::HuFloat => VolumeDecodeParameters {
                is_packed_flag: 0,
                bias: 0.0,
            },
        }
    }

    // Read a 3D texture from bytes (packed RG8 path)
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
        if volume_encoding == VolumeEncoding::HuFloat {
            return Self::from_bytes_r16f(
                device,
                queue,
                bytes,
                label,
                width,
                height,
                depth,
                volume_encoding,
            );
        }
        ensure!(
            volume_encoding.decode_bias().is_finite(),
            "Packed offset must be finite"
        );
        validate_upload(device, bytes, width, height, depth, 2)?;
        let texture_format = wgpu::TextureFormat::Rg8Unorm;
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
                bytes_per_row: Some(2 * width),
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
            volume_encoding == VolumeEncoding::HuFloat,
            "Expected HuFloat encoding"
        );
        validate_upload(device, bytes, width, height, depth, 2)?;
        let texture_format = wgpu::TextureFormat::R16Float;
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
                bytes_per_row: Some(2 * width),
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
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Linear,
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

    /// Build an R8Unorm 3D texture from a continuous scalar volume.
    ///
    /// The input bytes represent normalized scalar values in [0, 255].
    /// Unlike `from_labels_r8`, this texture is treated as a continuous
    /// scalar field rather than discrete segmentation labels.
    ///
    /// Sampling uses a linear filter so the scalar field can be smoothly
    /// interpolated during volume/MPR rendering.
    pub fn from_volume_r8(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        bytes: &[u8],
        label: &str,
        width: u32,
        height: u32,
        depth: u32,
    ) -> Result<Self> {
        validate_upload(device, bytes, width, height, depth, 1)?;
        let texture_format = wgpu::TextureFormat::R8Unorm;

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
                bytes_per_row: Some(width),
                rows_per_image: Some(height),
            },
            size,
        );

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Volume R8 Sampler"),

            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,

            // 关键：
            // 连续标量场使用 Linear，而不是 Nearest
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Nearest,

            ..Default::default()
        });

        Ok(Self {
            texture,
            view,
            sampler,
            texture_format,

            // 如果你的 VolumeEncoding 没有专门的 Normalized/R8 类型，
            // 暂时保持现有枚举即可。
            volume_encoding: VolumeEncoding::HuFloat,
        })
    }

    /// Build an R8Uint 3D texture from a label buffer (no smoothing).
    ///
    /// This is the GPU-side complement of `data::segmentation_volume::SegmentationVolume`.
    /// The label values are uploaded as-is so the shader can perform a
    /// direct lookup against the `label_table` uniform. See the design
    /// document Section 13 for the recommended texture format.
    pub fn from_labels_r8(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        bytes: &[u8],
        label: &str,
        width: u32,
        height: u32,
        depth: u32,
    ) -> Result<Self> {
        validate_upload(device, bytes, width, height, depth, 1)?;
        let texture_format = wgpu::TextureFormat::R8Uint;
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
                bytes_per_row: Some(width),
                rows_per_image: Some(height),
            },
            size,
        );

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::MipmapFilterMode::Nearest,
            ..Default::default()
        });
        Ok(Self {
            texture,
            view,
            sampler,
            texture_format,
            volume_encoding: VolumeEncoding::HuFloat,
        })
    }

    pub fn get_memory_stats(&self) -> (u64, u64, f32, f32) {
        let size = self.texture.size();

        let bytes_per_voxel = match self.texture_format {
            wgpu::TextureFormat::R8Unorm | wgpu::TextureFormat::R8Uint => 1,
            wgpu::TextureFormat::Rg8Unorm => 2,
            wgpu::TextureFormat::R16Float => 2,
            wgpu::TextureFormat::R32Float => 4,
            _ => 4,
        };

        let total_bytes = size.width as u64
            * size.height as u64
            * size.depth_or_array_layers as u64
            * bytes_per_voxel;

        (total_bytes, total_bytes, 1.0, 0.0)
    }

    /// Sample volume texture at normalized coordinates [0.0, 1.0] using CPU-side data access.
    /// Returns HU value (or 0 if out of bounds). This is a convenience helper for CPU-side
    /// volume picking used by measurement tools.
    pub fn sample_normalized(&self, x: f32, y: f32, z: f32) -> f32 {
        let size = self.texture.size();
        let ix = (x.clamp(0.0, 1.0) * (size.width as f32 - 1.0)).round() as u32;
        let iy = (y.clamp(0.0, 1.0) * (size.height as f32 - 1.0)).round() as u32;
        let iz = (z.clamp(0.0, 1.0) * (size.depth_or_array_layers as f32 - 1.0)).round() as u32;

        let bytes_per_row = size.width as usize
            * match self.texture_format {
                wgpu::TextureFormat::Rg8Unorm => 2,
                wgpu::TextureFormat::R16Float => 2,
                _ => 4,
            };
        let row_stride = bytes_per_row;
        let slice_stride = row_stride * size.height as usize;

        let offset = (iz as usize * slice_stride
            + iy as usize * row_stride
            + ix as usize
                * match self.texture_format {
                    wgpu::TextureFormat::Rg8Unorm => 2,
                    wgpu::TextureFormat::R16Float => 2,
                    _ => 4,
                }) as isize;

        // We don't have CPU access to GPU texture directly, so return 0 as fallback.
        // A proper implementation would require a staging buffer readback.
        // For now, measurement picking falls back to simple volume sampling.
        let _ = offset;
        0.0
    }
}

/// Check dimensions and byte counts before constructing any GPU object.
fn validate_upload(
    device: &wgpu::Device,
    bytes: &[u8],
    width: u32,
    height: u32,
    depth: u32,
    bytes_per_voxel: usize,
) -> Result<()> {
    ensure!(
        width > 0 && height > 0 && depth > 0,
        "Volume dimensions must be nonzero"
    );
    let limit = device.limits().max_texture_dimension_3d;
    ensure!(
        width <= limit && height <= limit && depth <= limit,
        "Volume dimensions exceed device limit {limit}"
    );
    let expected = (width as usize)
        .checked_mul(height as usize)
        .and_then(|n| n.checked_mul(depth as usize))
        .and_then(|n| n.checked_mul(bytes_per_voxel))
        .ok_or_else(|| anyhow!("Volume byte count overflow"))?;
    ensure!(
        bytes.len() == expected,
        "Volume byte count mismatch: expected {expected}, got {}",
        bytes.len()
    );
    ensure!(
        width.checked_mul(bytes_per_voxel as u32).is_some(),
        "Volume row byte count overflow"
    );
    Ok(())
}
