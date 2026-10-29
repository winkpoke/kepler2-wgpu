#![cfg(not(target_arch = "wasm32"))]
#[path = "common/headless_gpu.rs"]
mod headless_gpu;
use glam::{DMat3, DVec3};
use headless_gpu::Gpu;
use kepler_wgpu::{
    data::{beam::BeamOverlayGeometry, slice::SlicePlane},
    rendering::beam::{BeamInstance, BeamOverlayContext, BeamViewport},
};
type TestResult = Result<(), Box<dyn std::error::Error>>;

fn beam(angle: f64, selected: bool) -> BeamInstance {
    let rotation = DMat3::from_rotation_z(angle);
    BeamInstance {
        geometry: BeamOverlayGeometry {
            isocenter: DVec3::ZERO,
            x: rotation * DVec3::X,
            y: DVec3::Z,
            z: rotation * (-DVec3::Y),
            sad: 1000.,
            jaws: None,
            aperture: None,
        },
        selected,
        arc_directions: None,
    }
}
fn pixels(gpu: &Gpu, context: &BeamOverlayContext) -> Vec<u8> {
    let texture = gpu.device.create_texture(&wgpu::TextureDescriptor {
        label: Some("beam-batch-pixels"),
        size: wgpu::Extent3d {
            width: 384,
            height: 128,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let view = texture.create_view(&Default::default());
    let buffer = gpu.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("beam-batch-readback"),
        size: 384 * 128 * 4,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = gpu.device.create_command_encoder(&Default::default());
    {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("beam-batch-test"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        for index in 0..3 {
            pass.set_viewport(index as f32 * 128., 0., 128., 128., 0., 1.);
            context.render(&mut pass, index);
        }
    }
    encoder.copy_texture_to_buffer(
        texture.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(384 * 4),
                rows_per_image: Some(128),
            },
        },
        texture.size(),
    );
    gpu.queue.submit(Some(encoder.finish()));
    gpu.read_buffer(&buffer)
}

#[test]
fn multiple_beams_and_panes_keep_distinct_uniforms_and_reject_stale_batches() -> TestResult {
    let gpu = Gpu::new();
    let mut context = gpu.validated("Beam arrangement pipelines", || {
        BeamOverlayContext::new(&gpu.device, wgpu::TextureFormat::Rgba8Unorm)
    });
    let viewport = BeamViewport {
        plane: SlicePlane {
            origin: DVec3::new(-16., -16., 0.),
            right: DVec3::X,
            down: DVec3::Y,
            extent_mm: [32., 32.],
        },
        uv_origin: [0.; 2],
        uv_span: [1.; 2],
        pixels: [128.; 2],
        fields: false,
        axes: true,
        off_slice: false,
    };
    let beams = [beam(0., false), beam(std::f64::consts::FRAC_PI_2, true)];
    context.prepare(&gpu.device, &gpu.queue, 0, &beams, viewport)?;
    let mut shifted = beam(std::f64::consts::FRAC_PI_4, true);
    shifted.geometry.isocenter = DVec3::new(4., 4., 0.);
    context.prepare(&gpu.device, &gpu.queue, 1, &[shifted], viewport)?;
    let image = pixels(&gpu, &context);
    let lit = |x: usize, y: usize| image[(y * 384 + x) * 4..][..3].iter().any(|v| *v > 50);
    assert!(lit(64, 24), "first beam must survive second beam's upload");
    assert!(lit(104, 64), "second beam must keep its own direction");
    assert!(
        (30..70).any(|y| (190..240).any(|x| lit(x, y))),
        "second pane must retain its batch"
    );
    assert!(
        (0..128).all(|y| (256..384).all(|x| !lit(x, y))),
        "unused pane stays empty"
    );
    assert!(
        context
            .prepare(
                &gpu.device,
                &gpu.queue,
                0,
                &vec![beams[0].clone(); 129],
                viewport
            )
            .is_err()
    );
    let image = pixels(&gpu, &context);
    assert!(
        (0..128).all(|y| (0..128).all(|x| image[(y * 384 + x) * 4..][..3] == [0, 0, 0])),
        "rejected input clears all previous geometry"
    );
    assert!(
        context
            .prepare(
                &gpu.device,
                &gpu.queue,
                0,
                &beams,
                BeamViewport {
                    pixels: [0., 128.],
                    ..viewport
                }
            )
            .is_err()
    );
    Ok(())
}

#[test]
fn recorded_envelope_union_keeps_disjoint_fields_and_rejects_invalid_input() -> TestResult {
    use glam::DVec2;
    use kepler_wgpu::data::beam::{ApertureEdge, union_axis_aligned_outlines};
    use std::sync::Arc;
    let outlines: Vec<Arc<[ApertureEdge]>> = [[-6., -2.], [2., 6.], [3., 7.]]
        .into_iter()
        .map(|x| {
            Arc::from([
                ApertureEdge {
                    a: DVec2::new(x[0], -3.),
                    b: DVec2::new(x[0], 3.),
                },
                ApertureEdge {
                    a: DVec2::new(x[1], -3.),
                    b: DVec2::new(x[1], 3.),
                },
            ])
        })
        .collect();
    let union = union_axis_aligned_outlines(&outlines)?;
    assert_eq!(union.len(), 8);
    assert!(!union.iter().any(|s| s.a.x == 3. && s.b.x == 3.));
    assert!(!union.iter().any(|s| s.a.x < 0. && s.b.x > 0.));
    assert!(
        union_axis_aligned_outlines(&[Arc::from([ApertureEdge {
            a: DVec2::ZERO,
            b: DVec2::ONE
        }])])
        .is_err()
    );
    assert!(union_axis_aligned_outlines(&vec![Arc::from([]); 8193]).is_err());
    Ok(())
}
