//! Headless behavioral baseline for the production renderer on wgpu 23.
//! No ignored placeholders, window surfaces, or screenshot goldens.
#![cfg(not(target_arch = "wasm32"))]

#[path = "common/headless_gpu.rs"]
mod headless_gpu;

use glam::{Mat4, Vec3, Vec4};
use headless_gpu::{render_offscreen, Gpu, Image};
use kepler_wgpu::application::app_model::AppModel;
use kepler_wgpu::core::coord::Base;
use kepler_wgpu::data::ct_volume::CTVolume;
use kepler_wgpu::rendering::view::mesh::mesh::{BasicLightingUniforms, Mesh, MeshVertex};
use kepler_wgpu::rendering::view::mesh::BasicMeshContext;
use kepler_wgpu::rendering::view::mip::{MipRenderContext, MipUniforms, MipViewWgpuImpl};
use kepler_wgpu::rendering::view::mpr::{MprRenderContext, MprViewWgpuImpl};
use kepler_wgpu::rendering::view::render_content::RenderContent;
use kepler_wgpu::VolumeEncoding;
use std::sync::Arc;

const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
const ENCODINGS: [VolumeEncoding; 2] = [VolumeEncoding::PackedI16Rg8, VolumeEncoding::R16Float];

fn upload_volume(
    gpu: &Gpu,
    dims: (u32, u32, u32),
    voxels: Vec<i16>,
    encoding: VolumeEncoding,
) -> Arc<RenderContent> {
    assert_eq!(voxels.len(), (dims.0 * dims.1 * dims.2) as usize);
    // Use the application's CPU encoding and the production texture upload path.
    let mut model = AppModel::new(encoding == VolumeEncoding::R16Float);
    model
        .load_volume(CTVolume::new(
            (dims.0 as usize, dims.1 as usize, dims.2 as usize),
            (1.0, 1.0, 1.0),
            voxels,
            Base::default(),
        ))
        .unwrap();
    let (bytes, actual_encoding) = model.get_volume_render_data().unwrap();
    assert_eq!(actual_encoding, encoding);
    let content = RenderContent::from_bytes(
        &gpu.device,
        &gpu.queue,
        &bytes,
        "Synthetic CT volume",
        dims.0,
        dims.1,
        dims.2,
        actual_encoding,
    )
    .unwrap();
    assert_eq!(content.texture_format, encoding.texture_format());
    Arc::new(content)
}

fn draw_mpr(
    gpu: &Gpu,
    view: &MprViewWgpuImpl,
    width: u32,
    height: u32,
    format: wgpu::TextureFormat,
) -> Image {
    view.update_uniforms_buffers(&gpu.queue);
    render_offscreen(gpu, width, height, format, false, |pass| {
        let context = &view.render_context;
        pass.set_pipeline(&context.render_pipeline);
        pass.set_bind_group(0, &view.texture_bind_group, &[]);
        pass.set_bind_group(1, &view.uniform_vert_bind_group, &[]);
        pass.set_bind_group(2, &view.uniform_frag_bind_group, &[]);
        pass.set_vertex_buffer(0, context.vertex_buffer.slice(..));
        pass.set_index_buffer(context.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
        pass.draw_indexed(0..context.num_indices, 0, 0..1);
    })
}

fn draw_mip(
    gpu: &Gpu,
    content: Arc<RenderContent>,
    mut uniforms: MipUniforms,
    width: u32,
    height: u32,
    format: wgpu::TextureFormat,
) -> Image {
    let decode = content.decode_parameters();
    uniforms.is_packed_rg8 = decode.is_packed_flag as f32;
    uniforms.bias = decode.bias;
    let view = MipViewWgpuImpl::new(content, &gpu.device, format);
    view.update_uniforms(&gpu.queue, &uniforms);
    render_offscreen(gpu, width, height, format, false, |pass| {
        pass.set_pipeline(&view.render_context().pipeline);
        pass.set_bind_group(0, view.bind_groups().0, &[]);
        pass.set_bind_group(1, view.bind_groups().1, &[]);
        pass.draw(0..4, 0..1);
    })
}

fn mip_uniforms() -> MipUniforms {
    MipUniforms {
        ray_step_size: 0.125,
        max_steps: 9.0,
        window: 2001.0,
        level: 0.5,
        lower_threshold: -32768.0,
        upper_threshold: 32768.0,
        ..Default::default()
    }
}

/// Independent CPU oracle for DICOM linear window/level, not the shader decoder.
fn display_byte(hu: f32, width: f32, level: f32, format: wgpu::TextureFormat) -> u8 {
    let linear = ((hu - (level - 0.5)) / (width - 1.0) + 0.5).clamp(0.0, 1.0);
    color_byte(linear, format)
}

fn color_byte(linear: f32, format: wgpu::TextureFormat) -> u8 {
    let encoded = if format.is_srgb() {
        if linear <= 0.0031308 {
            12.92 * linear
        } else {
            1.055 * linear.powf(1.0 / 2.4) - 0.055
        }
    } else {
        linear
    };
    (encoded * 255.0).round() as u8
}

fn assert_gray(image: &Image, x: u32, y: u32, value: u8) {
    image.assert_pixel_near(x, y, [value, value, value, 255], 2);
}

/// Factories must target their own attachment, even when another factory was used first.
#[test]
fn factories_render_independent_surface_formats() {
    use kepler_wgpu::rendering::view::{DefaultViewFactory, Orientation, ViewFactory};
    let gpu = Gpu::new();
    gpu.validated("Independent factory formats", || {
        let vol = CTVolume::new((2, 2, 2), (1.0, 1.0, 1.0), vec![1000; 8], Base::default());
        for format in [
            wgpu::TextureFormat::Bgra8UnormSrgb,
            wgpu::TextureFormat::Rgba8UnormSrgb,
            FORMAT,
        ] {
            let factory = DefaultViewFactory::new(
                Arc::clone(&gpu.device),
                Arc::clone(&gpu.queue),
                format,
                false,
            );
            let mut mpr = factory
                .create_mpr_view(&vol, Orientation::Transverse, (0, 0), (16, 16))
                .unwrap();
            mpr.update(&gpu.queue);
            render_offscreen(&gpu, 16, 16, format, false, |pass| {
                mpr.render(pass).unwrap();
            });
            let mut mip = factory.create_mip_view(&vol, (0, 0), (16, 16)).unwrap();
            mip.update(&gpu.queue);
            render_offscreen(&gpu, 16, 16, format, false, |pass| {
                mip.render(pass).unwrap();
            });
            let mut mesh = factory
                .create_mesh_view(&overlapping_triangles(), (0, 0), (16, 16))
                .unwrap();
            mesh.update(&gpu.queue);
            render_offscreen(&gpu, 16, 16, format, true, |pass| {
                mesh.render(pass).unwrap();
            });
        }
    });
}

/// Existing views must remain usable after format and device replacement, with no state loss.
#[test]
fn existing_views_rebind_formats_and_devices_without_losing_display_state() {
    use kepler_wgpu::rendering::view::mesh::mesh_view::MeshView;
    use kepler_wgpu::rendering::view::{
        DefaultViewFactory, MipView, MprView, Orientation, ViewFactory,
    };
    let first = Gpu::new();
    let replacement = Gpu::new();
    let initial_format = wgpu::TextureFormat::Bgra8UnormSrgb;
    let next_format = wgpu::TextureFormat::Rgba8UnormSrgb;
    first.validated("Create views on original device", || {
        let vol = CTVolume::new(
            (2, 2, 2),
            (1.0, 1.0, 1.0),
            vec![-500, 0, 500, 1000, -500, 0, 500, 1000],
            Base::default(),
        );
        let content = upload_volume(
            &first,
            (2, 2, 2),
            vol.voxel_data().to_vec(),
            VolumeEncoding::PackedI16Rg8,
        );
        let factory = DefaultViewFactory::new(
            first.device.clone(),
            first.queue.clone(),
            initial_format,
            false,
        );
        let mut views = [
            factory
                .create_mpr_view_with_content(
                    content.clone(),
                    &vol,
                    Orientation::Transverse,
                    (0, 0),
                    (160, 160),
                )
                .unwrap(),
            factory
                .create_mip_view_with_content(content.clone(), (0, 0), (160, 160))
                .unwrap(),
            factory
                .create_mesh_view_with_content(
                    content,
                    &overlapping_triangles(),
                    (0, 0),
                    (160, 160),
                )
                .unwrap(),
        ];
        let mpr = views[0].as_any_mut().downcast_mut::<MprView>().unwrap();
        mpr.set_scale(1.2).unwrap();
        mpr.set_oblique_rotation_radians(0.1, 0.2, 0.3).unwrap();
        mpr.set_window_level(100.0).unwrap();
        let mip = views[1].as_any_mut().downcast_mut::<MipView>().unwrap();
        mip.set_rotation_radians(0.1, 0.2, 0.3);
        mip.config_mut().lower_threshold = -600.0;
        mip.config_mut().upper_threshold = 800.0;
        mip.set_scale(1.2);
        let mesh = views[2].as_any_mut().downcast_mut::<MeshView>().unwrap();
        mesh.set_scale_factor(0.8);
        mesh.set_opacity(0.7);
        let before: Vec<_> = views
            .iter_mut()
            .enumerate()
            .map(|(i, view)| {
                view.update(&first.queue);
                render_offscreen(&first, 160, 160, initial_format, i == 2, |pass| {
                    view.render(pass).unwrap()
                })
            })
            .collect();
        let expected_mip = views[1]
            .as_any()
            .downcast_ref::<MipView>()
            .unwrap()
            .config()
            .clone();
        // Same-device format change reuses volume textures. A replacement device gets new content,
        // uploaded on that device rather than reusing textures from the original device.
        for (gpu, content) in [
            (&first, None),
            (
                &replacement,
                Some(upload_volume(
                    &replacement,
                    (2, 2, 2),
                    vol.voxel_data().to_vec(),
                    VolumeEncoding::PackedI16Rg8,
                )),
            ),
        ] {
            gpu.validated("Rebind existing views", || {
                for (i, view) in views.iter_mut().enumerate() {
                    if let Some(mpr) = view.as_any_mut().downcast_mut::<MprView>() {
                        mpr.rebuild_gpu_resources(&gpu.device, next_format, content.clone());
                        assert_eq!(mpr.get_scale(), 1.2);
                        assert_eq!(mpr.get_window_level(), 100.0);
                    } else if let Some(mip) = view.as_any_mut().downcast_mut::<MipView>() {
                        mip.rebuild_gpu_resources(&gpu.device, next_format, content.clone());
                        assert_eq!(mip.config().lower_threshold, expected_mip.lower_threshold);
                        assert_eq!(mip.config().upper_threshold, expected_mip.upper_threshold);
                        assert_eq!(mip.get_rotation_radians(), [0.1, 0.2, 0.3]);
                    } else if let Some(mesh) = view.as_any_mut().downcast_mut::<MeshView>() {
                        mesh.rebuild_gpu_resources(&gpu.device, &gpu.queue, next_format);
                        assert_eq!(mesh.get_opacity(), 0.7);
                    }
                    view.update(&gpu.queue);
                    let after = render_offscreen(gpu, 160, 160, next_format, i == 2, |pass| {
                        view.render(pass).unwrap()
                    });
                    for (a, b) in after.pixels.iter().zip(&before[i].pixels) {
                        assert!(
                            a.abs_diff(*b) <= 2,
                            "View {i} changed after GPU rebinding: {a} != {b}"
                        );
                    }
                }
            });
        }
    });
}

#[test]
fn mpr_production_pipeline_builds() {
    let gpu = Gpu::new();
    gpu.validated("MPR production pipeline", || {
        let _context = MprRenderContext::new(&gpu.device, FORMAT);
    });
}

#[test]
fn mip_production_pipeline_builds() {
    let gpu = Gpu::new();
    gpu.validated("MIP production pipeline", || {
        let _context = MipRenderContext::new(&gpu.device, FORMAT);
    });
}

#[test]
fn mesh_production_pipeline_builds_with_and_without_depth() {
    let gpu = Gpu::new();
    gpu.validated("Mesh production pipelines", || {
        for use_depth in [false, true] {
            let _context = BasicMeshContext::new(
                &gpu.device,
                &gpu.queue,
                &overlapping_triangles(),
                use_depth,
                FORMAT,
            );
        }
    });
}

fn hu_render_roundtrip(encoding: VolumeEncoding) {
    const WINDOW_WIDTH: f32 = 65.0;
    let gpu = Gpu::new();
    gpu.validated("HU render roundtrip", || {
        let context = Arc::new(MprRenderContext::new(&gpu.device, FORMAT));
        for hu in [
            i16::MIN,
            -1200,
            -1100,
            -1024,
            -1,
            0,
            1000,
            2048,
            2049,
            3000,
            i16::MAX,
        ] {
            let content = upload_volume(&gpu, (1, 1, 1), vec![hu], encoding);
            let mut view = MprViewWgpuImpl::new(
                context.clone(),
                &gpu.device,
                content.clone(),
                Mat4::IDENTITY,
            );
            // Large half-float integers deliberately round. Pin that behavior too.
            let expected_hu = match (encoding, hu) {
                (VolumeEncoding::R16Float, 2049) => 2048.0,
                (VolumeEncoding::R16Float, i16::MAX) => 32768.0,
                _ => f32::from(hu),
            };
            // One output byte is about 0.25 HU. Even with quantization,
            // the ±1-byte tolerance cannot admit a two-HU decoding error.
            let level = expected_hu + 0.5;
            view.set_window_width(WINDOW_WIDTH);
            view.set_window_level(level);
            view.set_slice(0.5);
            let mpr = draw_mpr(&gpu, &view, 64, 64, FORMAT);
            let mip = draw_mip(
                &gpu,
                content,
                MipUniforms {
                    window: WINDOW_WIDTH,
                    level,
                    ..mip_uniforms()
                },
                64,
                64,
                FORMAT,
            );
            let expected = display_byte(expected_hu, WINDOW_WIDTH, level, FORMAT);
            for image in [&mpr, &mip] {
                for (x, y) in [(0, 0), (32, 32), (63, 63)] {
                    image.assert_pixel_near(x, y, [expected, expected, expected, 255], 1);
                }
            }
        }
    });
}

#[test]
fn packed_rg8_hu_values_roundtrip_through_mpr_and_mip_rendering() {
    hu_render_roundtrip(VolumeEncoding::PackedI16Rg8);
}

#[test]
fn r16float_hu_values_roundtrip_through_mpr_and_mip_rendering() {
    hu_render_roundtrip(VolumeEncoding::R16Float);
}

#[test]
fn mpr_axial_coronal_sagittal_slices_and_interpolation_match_voxels() {
    let gpu = Gpu::new();
    gpu.validated("MPR orthogonal sampling", || {
        // Each axis has a distinct slope, exposing swaps, flips and wrong slices.
        let mut voxels = Vec::new();
        for z in 0..4 {
            for y in 0..4 {
                for x in 0..4 {
                    voxels.push(-1200 + 100 * x + 200 * y + 400 * z);
                }
            }
        }
        let orientations = [
            Mat4::IDENTITY,                                      // axial: (u, v, slice)
            Mat4::from_cols(Vec4::X, Vec4::Z, Vec4::Y, Vec4::W), // coronal
            Mat4::from_cols(Vec4::Y, Vec4::Z, Vec4::X, Vec4::W), // sagittal
        ];
        let context = Arc::new(MprRenderContext::new(&gpu.device, FORMAT));
        for encoding in ENCODINGS {
            let content = upload_volume(&gpu, (4, 4, 4), voxels.clone(), encoding);
            for (plane, orientation) in orientations.into_iter().enumerate() {
                // Magnify 2x to exercise the production linear magnification
                // sampler. Even screen pixels map exactly to voxel centers.
                let matrix = orientation * Mat4::from_translation(Vec3::new(0.0625, 0.0625, 0.0));
                let mut view =
                    MprViewWgpuImpl::new(context.clone(), &gpu.device, content.clone(), matrix);
                view.set_window_width(2401.0);
                view.set_window_level(0.5);
                for (slice, voxel_slice) in [(0.375, 1.0), (0.625, 2.0), (0.5, 1.5)] {
                    view.set_slice(slice);
                    let image = draw_mpr(&gpu, &view, 8, 8, FORMAT);
                    for (u, v) in [(0, 0), (3, 0), (0, 3), (3, 3), (1, 2)] {
                        let (u, v) = (u as f32, v as f32);
                        let (x, y, z) = match plane {
                            0 => (u, v, voxel_slice),
                            1 => (u, voxel_slice, v),
                            _ => (voxel_slice, u, v),
                        };
                        let hu = -1200.0 + 100.0 * x + 200.0 * y + 400.0 * z;
                        assert_gray(
                            &image,
                            u as u32 * 2,
                            v as u32 * 2,
                            display_byte(hu, 2401.0, 0.5, FORMAT),
                        );
                    }
                }
                view.set_slice(-0.125);
                let image = draw_mpr(&gpu, &view, 8, 8, FORMAT);
                image.assert_pixel_near(2, 2, [0, 0, 0, 255], 0);
            }
        }
    });
}

#[test]
fn mip_selects_bright_voxel_and_preserves_negative_maxima() {
    let gpu = Gpu::new();
    gpu.validated("MIP maximum and signed intensity", || {
        for encoding in ENCODINGS {
            for peak in [1000, -400] {
                let mut voxels = vec![-900; 4 * 4 * 4];
                // One bright voxel along the ray at screen (2,1), not a bright slab.
                voxels[(2 * 4 + 1) * 4 + 2] = peak;
                let content = upload_volume(&gpu, (4, 4, 4), voxels, encoding);
                let image = draw_mip(&gpu, content, mip_uniforms(), 4, 4, FORMAT);
                assert_gray(
                    &image,
                    2,
                    1,
                    display_byte(f32::from(peak), 2001.0, 0.5, FORMAT),
                );
                for (x, y) in [(0, 0), (1, 1), (3, 3)] {
                    assert_gray(&image, x, y, display_byte(-900.0, 2001.0, 0.5, FORMAT));
                }
            }
        }
    });
}

#[test]
fn mip_minip_avgip_and_thresholds_match_known_ray_samples() {
    let gpu = Gpu::new();
    gpu.validated("Projection modes and thresholds", || {
        for encoding in ENCODINGS {
            let content = upload_volume(&gpu, (1, 1, 4), vec![-800, -200, 400, 1000], encoding);
            // Nine samples at z=0,1/8,...,1 with ClampToEdge + linear filtering:
            // [-800,-800,-500,-200,100,400,700,1000,1000].
            for (mode, expected) in [(0.0, 1000.0), (1.0, -800.0), (2.0, 100.0)] {
                let image = draw_mip(
                    &gpu,
                    content.clone(),
                    MipUniforms {
                        mip_mode: mode,
                        ..mip_uniforms()
                    },
                    4,
                    4,
                    FORMAT,
                );
                assert_gray(&image, 2, 2, display_byte(expected, 2001.0, 0.5, FORMAT));
            }
            // Filtering above 0 HU leaves [-800,-800,-500,-200].
            for (mode, expected) in [(0.0, -200.0), (1.0, -800.0), (2.0, -575.0)] {
                let image = draw_mip(
                    &gpu,
                    content.clone(),
                    MipUniforms {
                        mip_mode: mode,
                        upper_threshold: 0.0,
                        ..mip_uniforms()
                    },
                    4,
                    4,
                    FORMAT,
                );
                assert_gray(&image, 2, 2, display_byte(expected, 2001.0, 0.5, FORMAT));
            }
        }
    });
}

fn overlapping_triangles() -> Mesh {
    let mut vertices = Vec::new();
    for (z, color) in [(0.2, [1.0, 0.0, 0.0]), (0.8, [0.0, 0.0, 1.0])] {
        for (x, y) in [(-0.8, -0.8), (0.8, -0.8), (0.0, 0.8)] {
            vertices.push(MeshVertex {
                position: [x, y, z],
                normal: [0.0, 0.0, 1.0],
                color,
            });
        }
    }
    Mesh {
        vertices,
        indices: vec![0, 1, 2, 3, 4, 5],
    }
}

fn unshaded_lighting() -> BasicLightingUniforms {
    BasicLightingUniforms {
        light_direction: [0.0, 0.0, -1.0],
        ..Default::default()
    }
}

#[test]
fn mesh_depth_occludes_later_far_triangle() {
    let gpu = Gpu::new();
    gpu.validated("Mesh depth occlusion", || {
        for use_depth in [true, false] {
            let context = BasicMeshContext::new(
                &gpu.device,
                &gpu.queue,
                &overlapping_triangles(),
                use_depth,
                FORMAT,
            );
            context.update_lighting(&gpu.queue, unshaded_lighting());
            let image =
                render_offscreen(&gpu, 64, 64, FORMAT, use_depth, |pass| context.render(pass));
            let expected = if use_depth {
                [255, 0, 0, 255]
            } else {
                [0, 0, 255, 255]
            };
            // Without depth the later blue triangle wins; with depth red must win.
            image.assert_pixel_near(32, 32, expected, 0);
            image.assert_pixel_near(32, 40, expected, 0);
            image.assert_pixel_near(0, 0, [255, 0, 255, 0], 0);
        }
    });
}

#[test]
fn production_pipelines_render_rgba_bgra_and_srgb_targets_without_global_format_state() {
    let gpu = Gpu::new();
    gpu.validated("Output format matrix", || {
        let content = upload_volume(&gpu, (1, 1, 1), vec![0], VolumeEncoding::PackedI16Rg8);
        for format in [
            FORMAT,
            wgpu::TextureFormat::Bgra8Unorm,
            wgpu::TextureFormat::Rgba8UnormSrgb,
            wgpu::TextureFormat::Bgra8UnormSrgb,
        ] {
            let context = Arc::new(MprRenderContext::new_with_format(&gpu.device, format));
            let mut view =
                MprViewWgpuImpl::new(context, &gpu.device, content.clone(), Mat4::IDENTITY);
            view.set_window_width(257.0);
            view.set_window_level(0.5);
            let mpr = draw_mpr(&gpu, &view, 4, 4, format);
            let mip = draw_mip(
                &gpu,
                content.clone(),
                MipUniforms {
                    window: 257.0,
                    level: 0.5,
                    ..mip_uniforms()
                },
                4,
                4,
                format,
            );
            for image in [&mpr, &mip] {
                assert_gray(image, 2, 2, display_byte(0.0, 257.0, 0.5, format));
            }
            let mut mesh = overlapping_triangles();
            for vertex in &mut mesh.vertices {
                vertex.color = [0.25, 0.5, 0.75];
            }
            let context =
                BasicMeshContext::new_with_format(&gpu.device, &gpu.queue, &mesh, true, format);
            context.update_lighting(&gpu.queue, unshaded_lighting());
            let image = render_offscreen(&gpu, 64, 64, format, true, |pass| context.render(pass));
            image.assert_pixel_near(
                32,
                32,
                [
                    color_byte(0.25, format),
                    color_byte(0.5, format),
                    color_byte(0.75, format),
                    255,
                ],
                2,
            );
        }
    });
}
