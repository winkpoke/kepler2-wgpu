//! Import-to-render regressions with an asymmetric scalar phantom and independent geometry.
#![cfg(not(target_arch = "wasm32"))]
#[path = "common/headless_gpu.rs"]
mod headless_gpu;

use glam::{Mat4, Vec3};
use headless_gpu::{render_offscreen, Gpu};
use kepler_wgpu::{
    core::{geometry::GeometryBuilder, WindowLevel},
    data::medical_imaging::{import_nifti_bytes, ImportedNifti},
    rendering::view::{
        mesh::{
            mesh::{Mesh, MeshRenderContext, MeshVertex, VolumeUniforms},
            mesh_view::MeshView,
        },
        mpr::{MprRenderContext, MprView},
        Orientation, RenderContent, View,
    },
    Renderable, VolumeEncoding,
};
use std::sync::Arc;

fn phantom() -> ImportedNifti {
    let mut b = vec![0; 352];
    b[..4].copy_from_slice(&348_i32.to_le_bytes());
    for (offset, value) in [
        (40, 3_i16),
        (42, 4),
        (44, 4),
        (46, 4),
        (70, 4),
        (72, 16),
        (254, 1),
    ] {
        b[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
    }
    for (offset, value) in [(80, 1.0_f32), (84, 1.0), (88, 1.0), (108, 352.0)] {
        b[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }
    // RAS header -> LPS world = (10-y, 20+x, 30-z). Rotation and reflection.
    for (row, values) in [
        [0.0_f32, 1.0, 0.0, -10.0],
        [-1.0, 0.0, 0.0, -20.0],
        [0.0, 0.0, -1.0, 30.0],
    ]
    .into_iter()
    .enumerate()
    {
        for (col, value) in values.into_iter().enumerate() {
            let start = 280 + row * 16 + col * 4;
            b[start..start + 4].copy_from_slice(&value.to_le_bytes());
        }
    }
    b[123] = 2;
    b[344..348].copy_from_slice(b"n+1\0");
    for z in 0..4_i16 {
        for y in 0..4_i16 {
            for x in 0..4_i16 {
                b.extend_from_slice(&(x + 10 * y + 100 * z).to_le_bytes());
            }
        }
    }
    import_nifti_bytes(&b, Default::default()).unwrap()
}

fn content(gpu: &Gpu, imported: &ImportedNifti) -> Arc<RenderContent> {
    let encoding = VolumeEncoding::for_float_support(false);
    Arc::new(
        RenderContent::from_bytes(
            &gpu.device,
            &gpu.queue,
            &encoding.encode_volume(imported.volume.voxel_data()),
            "NIfTI phantom",
            4,
            4,
            4,
            encoding,
        )
        .unwrap(),
    )
}

#[test]
fn imported_volume_renders_world_oriented_mpr_and_matches_overlay_coordinates() {
    let gpu = Gpu::new();
    gpu.validated("NIfTI geometry through MPR rendering", || {
        let imported = phantom();
        let context = Arc::new(MprRenderContext::new_with_format(
            &gpu.device,
            &gpu.queue,
            wgpu::TextureFormat::Rgba8Unorm,
        ));
        let content = content(&gpu, &imported);
        let center = Vec3::new(8.5, 21.5, 28.5);
        for (orientation, right, down) in [
            (Orientation::Transverse, Vec3::X, Vec3::Y),
            (Orientation::Coronal, Vec3::X, -Vec3::Z),
            (Orientation::Sagittal, Vec3::Y, -Vec3::Z),
        ] {
            let mut wl = WindowLevel::new();
            wl.set_window_width(400.0).unwrap();
            wl.set_window_level(200.0).unwrap();
            wl.set_bias(0.0).unwrap();
            let mut view = MprView::new(
                context.clone(),
                &gpu.device,
                content.clone(),
                &imported.volume,
                orientation,
                wl,
                1.0,
                [0.0; 3],
                (0, 0),
                (64, 64),
            );
            // Isolate orientation/voxel selection from packed-RG8 filtering precision.
            // Interpolation and storage decode have their own volume_encoding regressions.
            view.set_aliasing(true);
            view.update(&gpu.queue);
            let image = render_offscreen(
                &gpu,
                64,
                64,
                wgpu::TextureFormat::Rgba8Unorm,
                false,
                |pass| view.render(pass).unwrap(),
            );
            for (x, y) in [(24, 24), (24, 40), (40, 24), (40, 40)] {
                let u = (x as f32 + 0.5) / 64.0;
                let v = (y as f32 + 0.5) / 64.0;
                let world = center + right * (4.0 * (u - 0.5)) + down * (4.0 * (v - 0.5));
                // Independent inverse of the fixture transform; no production affine in oracle.
                let voxel = Vec3::new(world.y - 20.0, 10.0 - world.x, 30.0 - world.z);
                let nearest = (voxel + Vec3::splat(0.5)).floor();
                let value = nearest.x + 10.0 * nearest.y + 100.0 * nearest.z;
                let gray = (((value - 199.5) / 399.0 + 0.5) * 255.0).round() as u8;
                image.assert_pixel_near(x, y, [gray, gray, gray, 255], 2);
                // Marker/mesh overlay and screen picking use the same physical location.
                assert!(Vec3::from_array(view.screen_coord_to_world([u, v, 0.0]))
                    .abs_diff_eq(world, 1e-5));
                assert!(
                    Vec3::from_array(view.world_coord_to_screen(world.to_array()))
                        .abs_diff_eq(Vec3::new(u, v, 0.0), 1e-5)
                );
                let uv = (voxel + Vec3::splat(0.5)) / 4.0;
                let (normal, d) = view.get_slice_plane_uv();
                assert!((normal.dot(uv) + d).abs() < 1e-5);
            }
            let vertices = [[0.4, 0.4], [0.6, 0.4], [0.5, 0.6]]
                .map(|[u, v]| {
                    let world = center + right * (4.0 * (u - 0.5)) + down * (4.0 * (v - 0.5));
                    let voxel = Vec3::new(world.y - 20.0, 10.0 - world.x, 30.0 - world.z);
                    MeshVertex {
                        position: ((voxel + Vec3::splat(0.5)) / 4.0).to_array(),
                        normal: [0.0, 0.0, 1.0],
                        color: [1.0, 0.0, 0.0],
                    }
                })
                .to_vec();
            view.set_mesh(
                1,
                0,
                &gpu.device,
                &gpu.queue,
                Arc::new(vec![Mesh {
                    vertices,
                    indices: vec![0, 2, 1],
                    label_id: 0,
                    label_name: String::new(),
                }]),
            );
            view.set_mesh_overlay(true, 0.02);
            view.update(&gpu.queue);
            let overlay = render_offscreen(
                &gpu,
                64,
                64,
                wgpu::TextureFormat::Rgba8Unorm,
                false,
                |pass| view.render(pass).unwrap(),
            );
            let pixel = &overlay.pixels[((30 * 64 + 32) * 4)..((30 * 64 + 32) * 4 + 4)];
            assert!(
                pixel[0] > pixel[1] + 10 && pixel[1] == pixel[2],
                "mesh must overlay its physical slice location: {pixel:?}"
            );
        }
    });
}

#[test]
fn imported_dvr_uses_the_same_physical_transform_as_its_uv_space_meshes() {
    let gpu = Gpu::new();
    gpu.validated("NIfTI DVR geometry", || {
        let imported = phantom();
        let content = content(&gpu, &imported);
        let context = Arc::new(MeshRenderContext::new(
            &gpu.device,
            wgpu::TextureFormat::Rgba8Unorm,
            content,
        ));
        let mut view = MeshView::new();
        view.attach_context(context.clone());
        view.set_volume_geometry(&imported.volume);
        view.resize((64, 64));
        view.set_window_width(2000.0).unwrap();
        view.set_window_level(0.0).unwrap();
        view.update(&gpu.queue);
        let actual = render_offscreen(
            &gpu,
            64,
            64,
            wgpu::TextureFormat::Rgba8Unorm,
            true,
            |pass| view.render(pass).unwrap(),
        );
        // Build a separate context with an independently written normalized fixture transform.
        let model = Mat4::from_cols(
            Vec3::Y.extend(0.0),
            (-Vec3::X).extend(0.0),
            (-Vec3::Z).extend(0.0),
            Vec3::new(1.0, 0.0, 1.0).extend(1.0),
        );
        assert!(
            GeometryBuilder::normalized_texture_to_world(&imported.volume).abs_diff_eq(model, 1e-5)
        );
        let camera = view.camera();
        let vp = camera.view_projection_matrix(1.0) * model;
        let decode = context.render_content.decode_parameters();
        context.update_uniforms(
            &gpu.queue,
            &VolumeUniforms {
                ray_step_size: 0.004,
                max_steps: 1500.0,
                is_packed_rg8: decode.is_packed_flag as f32,
                bias: decode.bias,
                window: 2000.0,
                level: 0.0,
                opacity: view.get_opacity(),
                volume_scale: [1.0; 3],
                vol_dims: [4.0; 3],
                view_proj: vp.to_cols_array(),
                inv_view_proj: vp.inverse().to_cols_array(),
                camera_position: model
                    .inverse()
                    .transform_point3(camera.effective_position())
                    .to_array(),
                light_dir: [0.5, 0.5, -1.0],
                plane_rotation_angle: 180.0,
                ..Default::default()
            },
        );
        let expected = render_offscreen(
            &gpu,
            64,
            64,
            wgpu::TextureFormat::Rgba8Unorm,
            true,
            |pass| context.render(pass),
        );
        assert!(
            actual.pixels.chunks_exact(4).any(|p| p[0] > 0),
            "DVR must contain visible volume"
        );
        for (a, b) in actual.pixels.iter().zip(&expected.pixels) {
            assert!(
                a.abs_diff(*b) <= 2,
                "DVR differs from physical transform oracle: {a} vs {b}"
            );
        }
    });
}
