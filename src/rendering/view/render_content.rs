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
            wgpu::ImageCopyTexture {
                aspect: wgpu::TextureAspect::All,
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
            },
            bytes,
            wgpu::ImageDataLayout {
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
            mipmap_filter: wgpu::FilterMode::Nearest,
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
