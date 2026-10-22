#![cfg(not(target_arch = "wasm32"))]
#[path = "common/headless_gpu.rs"]
mod headless_gpu;
use glam::{DMat4, DVec2, DVec3};
use headless_gpu::Gpu;
use kepler_wgpu::{
    data::{
        bev::{BeamProjection, DetectorRect, VolumeGeometry},
        volume_encoding::VolumeEncoding,
    },
    rendering::{
        RenderContent,
        bev::{BevRenderContext, BevRenderRequest},
    },
};
use std::sync::Arc;
type TestResult = Result<(), Box<dyn std::error::Error>>;

fn read_color(gpu: &Gpu, texture: &wgpu::Texture) -> Vec<u8> {
    let size = texture.size();
    let row = (size.width * 4).div_ceil(256) * 256;
    let buffer = gpu.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("bev-test-readback"),
        size: u64::from(row) * u64::from(size.height),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = gpu.device.create_command_encoder(&Default::default());
    encoder.copy_texture_to_buffer(
        texture.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(row),
                rows_per_image: Some(size.height),
            },
        },
        size,
    );
    gpu.queue.submit(Some(encoder.finish()));
    let bytes = gpu.read_buffer(&buffer);
    let mut out = Vec::new();
    for line in bytes.chunks_exact(row as usize) {
        out.extend_from_slice(&line[..size.width as usize * 4]);
    }
    out
}

#[test]
fn bev_gpu_matches_full_physical_slabs_for_both_encodings_and_low_step_budget() -> TestResult {
    let gpu = Gpu::new();
    for float in [false, true] {
        let encoding = VolumeEncoding::for_float_support(float);
        let volume = Arc::new(RenderContent::from_bytes(
            &gpu.device,
            &gpu.queue,
            &encoding.encode_volume(&vec![0; 8 * 12 * 6]),
            "bev-slab",
            8,
            12,
            6,
            encoding,
        )?);
        let geometry = VolumeGeometry::new(
            [8, 12, 6],
            DVec3::ZERO,
            [DVec3::X * 2., DVec3::Y * 3., DVec3::Z * 4.],
            DMat4::IDENTITY,
        )?;
        let beam = BeamProjection::new(
            DVec3::new(7., 16.5, 10.),
            DVec3::X,
            DVec3::Z,
            -DVec3::Y,
            1000.,
        )?;
        let mut context = gpu.validated("BEV pipelines", || {
            BevRenderContext::new(&gpu.device, Arc::clone(&volume))
        })?;
        for max_steps in [1, 256] {
            let request = BevRenderRequest {
                beam,
                volume: &geometry,
                detector: DetectorRect::new(DVec2::new(-16., 16.), DVec2::splat(32.))?,
                size: [64, 64],
                max_steps,
                contrast: 0.02,
            };
            gpu.validated("BEV slab", || {
                let mut encoder = gpu.device.create_command_encoder(&Default::default());
                context.render(&gpu.device, &gpu.queue, &mut encoder, &request)?;
                gpu.queue.submit(Some(encoder.finish()));
                Ok::<_, kepler_wgpu::core::KeplerError>(())
            })?;
            let pixels = read_color(&gpu, context.color_texture().ok_or("color")?);
            let observed = pixels[(32 * 64 + 32) * 4];
            let expected = 255. * (1. - (-36. * 0.02_f32).exp());
            assert!(
                (f32::from(observed) - expected).abs() <= 2.,
                "float={float}, steps={max_steps}, {observed} vs {expected}"
            );
            assert_eq!(pixels[(2 * 64 + 2) * 4], 0);
        }
        let integrations = context.stats().integrations;
        let mut request = BevRenderRequest {
            beam,
            volume: &geometry,
            detector: DetectorRect::new(DVec2::new(-16., 16.), DVec2::splat(32.))?,
            size: [64, 64],
            max_steps: 256,
            contrast: 0.03,
        };
        let mut encoder = gpu.device.create_command_encoder(&Default::default());
        context.render(&gpu.device, &gpu.queue, &mut encoder, &request)?;
        gpu.queue.submit(Some(encoder.finish()));
        assert_eq!(
            context.stats().integrations,
            integrations,
            "Contrast must reuse the physical integral"
        );
        request.beam = BeamProjection::new(beam.isocenter(), DVec3::Y, DVec3::Z, DVec3::X, 1000.)?;
        let mut encoder = gpu.device.create_command_encoder(&Default::default());
        context.render(&gpu.device, &gpu.queue, &mut encoder, &request)?;
        gpu.queue.submit(Some(encoder.finish()));
        let pixels = read_color(&gpu, context.color_texture().ok_or("color")?);
        let expected = 255. * (1. - (-16. * 0.03_f32).exp());
        assert!((f32::from(pixels[(32 * 64 + 32) * 4]) - expected).abs() <= 2.);
        assert_eq!(context.stats().integrations, integrations + 1);
        let generation = context.target_generation();
        context.prepare_target(&gpu.device, [32, 32])?;
        assert_ne!(context.target_generation(), generation);
        assert_eq!(context.target_bytes(), 32 * 32 * 8);
        assert!(context.prepare_target(&gpu.device, [2048, 1]).is_err());
        assert_eq!(
            Arc::strong_count(&volume),
            2,
            "The context retains exactly the shared upload"
        );
    }
    Ok(())
}

#[test]
fn bev_gpu_keeps_air_black_and_rejects_invalid_requests() -> TestResult {
    let gpu = Gpu::new();
    let encoding = VolumeEncoding::for_float_support(false);
    let volume = Arc::new(RenderContent::from_bytes(
        &gpu.device,
        &gpu.queue,
        &encoding.encode_volume(&[-1000; 64]),
        "air",
        4,
        4,
        4,
        encoding,
    )?);
    let mut context = BevRenderContext::new(&gpu.device, volume)?;
    let geometry = VolumeGeometry::new(
        [4, 4, 4],
        DVec3::ZERO,
        [DVec3::X, DVec3::Y, DVec3::Z],
        DMat4::IDENTITY,
    )?;
    let beam = BeamProjection::new(DVec3::splat(1.5), DVec3::X, DVec3::Z, -DVec3::Y, 1000.)?;
    let mut request = BevRenderRequest {
        beam,
        volume: &geometry,
        detector: DetectorRect::new(DVec2::new(-2., 2.), DVec2::splat(4.))?,
        size: [32, 32],
        max_steps: 256,
        contrast: 0.02,
    };
    let mut encoder = gpu.device.create_command_encoder(&Default::default());
    context.render(&gpu.device, &gpu.queue, &mut encoder, &request)?;
    gpu.queue.submit(Some(encoder.finish()));
    let pixels = read_color(&gpu, context.color_texture().ok_or("color")?);
    assert!(pixels.chunks_exact(4).all(|pixel| pixel == [0, 0, 0, 255]));
    request.contrast = f32::NAN;
    let mut encoder = gpu.device.create_command_encoder(&Default::default());
    assert!(
        context
            .render(&gpu.device, &gpu.queue, &mut encoder, &request)
            .is_err()
    );
    Ok(())
}

#[test]
fn bev_gpu_overlay_draws_physical_aperture_on_the_host_pass() -> TestResult {
    use kepler_wgpu::rendering::bev_overlay::{BevOverlayContext, BevOverlayScene};
    let gpu = Gpu::new();
    let mut context = gpu.validated("BEV overlay pipeline", || {
        BevOverlayContext::new(&gpu.device, wgpu::TextureFormat::Rgba8Unorm)
    });
    let mut scene = BevOverlayScene::default();
    scene.jaws([-5., 5., -5., 5.], true)?;
    context.prepare(
        &gpu.queue,
        &Arc::new(scene),
        DetectorRect::new(DVec2::new(-16., 16.), DVec2::splat(32.))?,
        [64., 64.],
    )?;
    let image = gpu.validated("BEV aperture", || {
        headless_gpu::render_offscreen(
            &gpu,
            64,
            64,
            wgpu::TextureFormat::Rgba8Unorm,
            false,
            |pass| context.render(pass),
        )
    });
    image.assert_pixel_near(22, 32, [92, 222, 235, 255], 20);
    image.assert_pixel_near(41, 32, [92, 222, 235, 255], 20);
    image.assert_pixel_near(32, 32, [255, 0, 255, 0], 0);
    Ok(())
}
