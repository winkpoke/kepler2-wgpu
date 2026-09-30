//! Native, fail-closed GPU test utilities. No window or surface is created.

use std::sync::mpsc;
use std::time::Duration;

pub struct Gpu {
    pub device: std::sync::Arc<wgpu::Device>,
    pub queue: std::sync::Arc<wgpu::Queue>,
    pub adapter_info: wgpu::AdapterInfo,
}

impl Gpu {
    pub fn new() -> Self {
        let _ = env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn"))
            .is_test(true)
            .try_init();
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::util::backend_bits_from_env().unwrap_or(wgpu::Backends::PRIMARY),
            flags: wgpu::InstanceFlags::VALIDATION,
            ..Default::default()
        });
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            compatible_surface: None,
            force_fallback_adapter: false,
            power_preference: wgpu::PowerPreference::LowPower,
        }))
        .expect("Headless GPU tests require an adapter; install Mesa lavapipe on Linux and set WGPU_BACKEND=vulkan. These tests must not silently skip.");
        let adapter_info = adapter.get_info();
        log::info!("Headless test adapter: {adapter_info:?}");
        let (device, queue) =
            pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default(), None))
                .unwrap_or_else(|error| panic!("Requesting device on {adapter_info:?}: {error}"));
        Self {
            device: std::sync::Arc::new(device),
            queue: std::sync::Arc::new(queue),
            adapter_info,
        }
    }

    /// Captures validation failures, including those deferred until submission.
    pub fn validated<T>(&self, label: &str, run: impl FnOnce() -> T) -> T {
        self.device.push_error_scope(wgpu::ErrorFilter::Validation);
        let result = run();
        self.device.poll(wgpu::Maintain::Wait);
        let error = pollster::block_on(self.device.pop_error_scope());
        assert!(
            error.is_none(),
            "{label} on {:?}: {error:?}",
            self.adapter_info
        );
        result
    }

    pub fn read_buffer(&self, buffer: &wgpu::Buffer) -> Vec<u8> {
        let (tx, rx) = mpsc::channel();
        buffer
            .slice(..)
            .map_async(wgpu::MapMode::Read, move |result| {
                let _ = tx.send(result);
            });
        self.device.poll(wgpu::Maintain::Wait);
        rx.recv_timeout(Duration::from_secs(30))
            .expect("Timed out waiting for GPU readback")
            .expect("GPU readback mapping failed");
        let bytes = buffer.slice(..).get_mapped_range().to_vec();
        buffer.unmap();
        bytes
    }
}

pub struct Image {
    pub width: u32,
    pub height: u32,
    /// Always RGBA, even for BGRA render targets. sRGB bytes remain sRGB encoded.
    pub pixels: Vec<u8>,
}

impl Image {
    pub fn assert_pixel_near(&self, x: u32, y: u32, expected: [u8; 4], tolerance: u8) {
        assert!(x < self.width && y < self.height);
        let offset = ((y * self.width + x) * 4) as usize;
        let actual = &self.pixels[offset..offset + 4];
        for (channel, (&actual, &expected)) in actual.iter().zip(&expected).enumerate() {
            assert!(
                actual.abs_diff(expected) <= tolerance,
                "Pixel ({x}, {y}), channel {channel}: {actual} != {expected} (tolerance {tolerance})"
            );
        }
    }
}

/// Draws into a color attachment and reads it back with correctly padded rows.
pub fn render_offscreen(
    gpu: &Gpu,
    width: u32,
    height: u32,
    format: wgpu::TextureFormat,
    use_depth: bool,
    draw: impl FnOnce(&mut wgpu::RenderPass<'_>),
) -> Image {
    let size = wgpu::Extent3d {
        width,
        height,
        depth_or_array_layers: 1,
    };
    let texture = gpu.device.create_texture(&wgpu::TextureDescriptor {
        label: Some("Headless color target"),
        size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let color_view = texture.create_view(&Default::default());
    let depth_texture = use_depth.then(|| {
        gpu.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Headless mesh depth"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: kepler_wgpu::rendering::core::pipeline::get_mesh_depth_format(),
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        })
    });
    let depth_view = depth_texture
        .as_ref()
        .map(|texture| texture.create_view(&Default::default()));
    let unpadded_row = width * 4;
    let padded_row = unpadded_row.div_ceil(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT)
        * wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
    let readback = gpu.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Headless pixel readback"),
        size: u64::from(padded_row) * u64::from(height),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = gpu.device.create_command_encoder(&Default::default());
    {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Headless rendering correctness"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &color_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    // Non-grayscale clear makes missing/partial draws visible to assertions.
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 1.0,
                        g: 0.0,
                        b: 1.0,
                        a: 0.0,
                    }),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: depth_view.as_ref().map(|view| {
                wgpu::RenderPassDepthStencilAttachment {
                    view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        draw(&mut pass);
    }
    encoder.copy_texture_to_buffer(
        wgpu::ImageCopyTexture {
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::ImageCopyBuffer {
            buffer: &readback,
            layout: wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(padded_row),
                rows_per_image: Some(height),
            },
        },
        size,
    );
    gpu.queue.submit(Some(encoder.finish()));
    let mapped = gpu.read_buffer(&readback);
    let mut pixels = Vec::with_capacity((unpadded_row * height) as usize);
    for row in mapped.chunks_exact(padded_row as usize) {
        pixels.extend_from_slice(&row[..unpadded_row as usize]);
    }
    if matches!(
        format,
        wgpu::TextureFormat::Bgra8Unorm | wgpu::TextureFormat::Bgra8UnormSrgb
    ) {
        for pixel in pixels.as_chunks_mut::<4>().0 {
            pixel.swap(0, 2);
        }
    }
    Image {
        width,
        height,
        pixels,
    }
}
