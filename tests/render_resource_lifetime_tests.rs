//! GPU resource lifetime and event-driven update regressions, using production paths.
#![cfg(not(target_arch = "wasm32"))]

#[path = "common/headless_gpu.rs"]
mod headless_gpu;

use headless_gpu::{render_offscreen, Gpu};
use kepler_wgpu::core::coord::Base;
use kepler_wgpu::data::ct_volume::CTVolume;
use kepler_wgpu::rendering::core::PassExecutor;
use kepler_wgpu::rendering::view::mesh::mesh_texture_pool::MeshTexturePool;
use kepler_wgpu::rendering::view::{
    DefaultViewFactory, MeshView, MipView, MprView, Orientation, ViewFactory,
};
use kepler_wgpu::View;
use std::sync::Arc;

const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;

fn factory(gpu: &Gpu, format: wgpu::TextureFormat) -> DefaultViewFactory {
    DefaultViewFactory::new(gpu.device.clone(), gpu.queue.clone(), format, false)
}

fn volume() -> CTVolume {
    CTVolume::new((2, 2, 2), (1.0, 1.0, 1.0), vec![500; 8], Base::default())
}

#[test]
fn repeated_mesh_passes_reuse_one_depth_texture_and_view() {
    let gpu = Gpu::new();
    gpu.validated("Repeated mesh pass depth reuse", || {
        let mut pool = MeshTexturePool::new();
        let mut executor = PassExecutor::new(FORMAT);
        let target = gpu.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Simulated surface"),
            size: wgpu::Extent3d {
                width: 16,
                height: 16,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let view = target.create_view(&Default::default());
        for _ in 0..60 {
            let mut encoder = gpu.device.create_command_encoder(&Default::default());
            executor
                .execute_frame(
                    &mut encoder,
                    &view,
                    &mut pool,
                    &gpu.device,
                    16,
                    16,
                    true,
                    false,
                    false,
                    |_| Ok(()),
                )
                .unwrap();
            gpu.queue.submit(Some(encoder.finish()));
        }
        assert_eq!(
            pool.allocation_count(),
            1,
            "one depth texture/view pair for 60 frames"
        );
        assert!(pool.depth_view().is_some());
    });
}

#[test]
fn depth_cache_keys_resize_format_samples_and_zero_size() {
    let gpu = Gpu::new();
    gpu.validated("Depth cache invalidation", || {
        let mut pool = MeshTexturePool::new();
        let depth = wgpu::TextureFormat::Depth24Plus;
        assert!(pool.ensure_depth(&gpu.device, 16, 16, 1, depth).is_some());
        assert_eq!(pool.allocation_count(), 1);
        assert!(pool.ensure_depth(&gpu.device, 16, 16, 1, depth).is_some());
        assert_eq!(pool.allocation_count(), 1);
        pool.ensure_depth(&gpu.device, 32, 16, 1, depth);
        pool.ensure_depth(&gpu.device, 32, 32, 1, depth);
        pool.ensure_depth(&gpu.device, 32, 32, 1, wgpu::TextureFormat::Depth32Float);
        pool.ensure_depth(&gpu.device, 32, 32, 4, wgpu::TextureFormat::Depth32Float);
        assert_eq!(pool.allocation_count(), 5);
        assert!(pool.ensure_depth(&gpu.device, 0, 32, 1, depth).is_none());
        assert!(pool.ensure_depth(&gpu.device, 32, 0, 1, depth).is_none());
        assert_eq!(pool.allocation_count(), 5);
        pool.clear_depth_view();
        assert!(pool.depth_view().is_none());
        pool.ensure_depth(&gpu.device, 16, 16, 1, depth);
        assert_eq!(pool.allocation_count(), 6);
    });
}

#[test]
fn compatibility_helpers_adopt_and_reuse_depth_metadata() {
    let gpu = Gpu::new();
    gpu.validated("Depth compatibility helpers", || {
        let mut pool = MeshTexturePool::new();
        pool.ensure_textures(&gpu.device, 16, 16, false);
        assert!(pool.depth_view().is_none());
        pool.ensure_textures(&gpu.device, 16, 16, true);
        pool.ensure_textures(&gpu.device, 16, 16, true);
        assert_eq!(pool.allocation_count(), 1);
        let texture = gpu.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Adopted depth"),
            size: wgpu::Extent3d {
                width: 32,
                height: 16,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth24Plus,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let view = texture.create_view(&Default::default());
        pool.set_depth(texture, view);
        pool.ensure_depth(&gpu.device, 32, 16, 1, wgpu::TextureFormat::Depth24Plus);
        assert_eq!(
            pool.allocation_count(),
            1,
            "adopted attachment must be reused"
        );
        pool.ensure_depth(&gpu.device, 16, 16, 1, wgpu::TextureFormat::Depth24Plus);
        assert_eq!(pool.allocation_count(), 2);
    });
}

#[test]
fn failed_pass_is_not_reported_as_a_successfully_presentable_frame() {
    let gpu = Gpu::new();
    gpu.validated("Failed frame acknowledgement", || {
        let mut executor = PassExecutor::new(FORMAT);
        let mut pool = MeshTexturePool::new();
        headless_gpu::render_encoded_offscreen(&gpu, 16, 16, FORMAT, |encoder, target| {
            let result = executor.execute_frame(
                encoder,
                target,
                &mut pool,
                &gpu.device,
                16,
                16,
                false,
                true,
                false,
                |_| Err(Box::new(std::io::Error::other("Injected render failure"))),
            );
            assert!(
                result.is_err(),
                "App must not clear pending presentation after a failed pass"
            );
        });
    });
}

#[test]
fn mpr_factory_shares_context_across_creation_paths_and_rebinding() {
    let gpu = Gpu::new();
    gpu.validated("Shared MPR factory context", || {
        let factory = factory(&gpu, FORMAT);
        let vol = volume();
        let mut first = factory
            .create_mpr_view(&vol, Orientation::Transverse, (0, 0), (16, 16))
            .unwrap();
        let context = first
            .as_any()
            .downcast_ref::<MprView>()
            .unwrap()
            .render_context()
            .clone();
        let content = first
            .as_any()
            .downcast_ref::<MprView>()
            .unwrap()
            .render_content()
            .clone();
        let mut second = factory
            .create_mpr_view_with_content(content, &vol, Orientation::Coronal, (0, 0), (16, 16))
            .unwrap();
        assert!(Arc::ptr_eq(
            &context,
            second
                .as_any()
                .downcast_ref::<MprView>()
                .unwrap()
                .render_context()
        ));
        first.resize((32, 32));
        assert!(Arc::ptr_eq(
            &context,
            first
                .as_any()
                .downcast_ref::<MprView>()
                .unwrap()
                .render_context()
        ));
        let replacement = super_factory(&gpu);
        for view in [&mut first, &mut second] {
            let mpr = view.as_any_mut().downcast_mut::<MprView>().unwrap();
            mpr.rebuild_gpu_resources_with_context(
                &gpu.device,
                replacement.mpr_render_context(),
                None,
            );
            assert!(Arc::ptr_eq(
                &replacement.mpr_render_context(),
                mpr.render_context()
            ));
            assert!(!Arc::ptr_eq(&context, mpr.render_context()));
            view.update(&gpu.queue);
            render_offscreen(
                &gpu,
                32,
                32,
                wgpu::TextureFormat::Bgra8Unorm,
                false,
                |pass| view.render(pass).unwrap(),
            );
        }
    });
}

fn super_factory(gpu: &Gpu) -> DefaultViewFactory {
    factory(gpu, wgpu::TextureFormat::Bgra8Unorm)
}

#[test]
fn mpr_dirty_state_survives_update_until_presentation_and_ignores_noops() {
    let gpu = Gpu::new();
    gpu.validated("MPR presentation invalidation", || {
        let factory = factory(&gpu, FORMAT);
        let mut view = factory
            .create_mpr_view(&volume(), Orientation::Transverse, (0, 0), (16, 16))
            .unwrap();
        assert!(view.needs_redraw());
        view.update(&gpu.queue);
        assert!(view.needs_redraw(), "upload is not presentation");
        view.mark_presented();
        assert!(!view.needs_redraw());
        let mpr = view.as_any_mut().downcast_mut::<MprView>().unwrap();
        mpr.set_scale(mpr.get_scale()).unwrap();
        mpr.set_window_level(mpr.get_window_level()).unwrap();
        assert!(!view.needs_redraw());
        view.as_any_mut()
            .downcast_mut::<MprView>()
            .unwrap()
            .set_slice_mm(0.25)
            .unwrap();
        assert!(view.needs_redraw());
        view.update(&gpu.queue);
        assert!(
            view.needs_redraw(),
            "failed frame must retain pending presentation"
        );
        view.mark_presented();
        assert!(!view.needs_redraw());
    });
}

#[test]
fn mip_config_mut_and_mesh_controls_invalidate_without_a_setter_audit_gap() {
    let gpu = Gpu::new();
    gpu.validated("MIP and mesh invalidation", || {
        let factory = factory(&gpu, FORMAT);
        let mut mip = factory
            .create_mip_view(&volume(), (0, 0), (16, 16))
            .unwrap();
        mip.update(&gpu.queue);
        mip.mark_presented();
        assert!(!mip.needs_redraw());
        mip.as_any_mut()
            .downcast_mut::<MipView>()
            .unwrap()
            .config_mut()
            .slab_thickness = 20.0;
        assert!(mip.needs_redraw());
        let mut mesh = factory
            .create_mesh_view(
                &kepler_wgpu::mesh::mesh::Mesh::unit_cube(),
                (0, 0),
                (16, 16),
            )
            .unwrap();
        mesh.update(&gpu.queue);
        mesh.mark_presented();
        assert!(!mesh.needs_redraw());
        let concrete = mesh.as_any_mut().downcast_mut::<MeshView>().unwrap();
        concrete.set_opacity(0.5);
        assert!(mesh.needs_redraw());
        mesh.update(&gpu.queue);
        mesh.mark_presented();
        mesh.as_any_mut()
            .downcast_mut::<MeshView>()
            .unwrap()
            .set_opacity(0.5);
        assert!(!mesh.needs_redraw());
        mesh.as_any_mut()
            .downcast_mut::<MeshView>()
            .unwrap()
            .set_rotation_enabled(true);
        assert!(mesh.is_animating());
        mesh.as_any_mut()
            .downcast_mut::<MeshView>()
            .unwrap()
            .set_rotation_enabled(false);
        assert!(!mesh.is_animating());
    });
}

#[test]
fn unchanged_mpr_view_is_still_drawn_into_each_fresh_frame() {
    let gpu = Gpu::new();
    gpu.validated("Complete frames with independent uniforms", || {
        let factory = factory(&gpu, FORMAT);
        let vol = volume();
        let mut left = factory
            .create_mpr_view(&vol, Orientation::Transverse, (0, 0), (16, 16))
            .unwrap();
        let content = left
            .as_any()
            .downcast_ref::<MprView>()
            .unwrap()
            .render_content()
            .clone();
        let mut right = factory
            .create_mpr_view_with_content(content, &vol, Orientation::Transverse, (16, 0), (16, 16))
            .unwrap();
        left.update(&gpu.queue);
        right.update(&gpu.queue);
        let draw = |left: &mut Box<dyn View>, right: &mut Box<dyn View>| {
            render_offscreen(&gpu, 32, 16, FORMAT, false, |pass| {
                left.render(pass).unwrap();
                right.render(pass).unwrap();
            })
        };
        let before = draw(&mut left, &mut right);
        left.mark_presented();
        right.mark_presented();
        left.as_any_mut()
            .downcast_mut::<MprView>()
            .unwrap()
            .set_window_level(-1000.0)
            .unwrap();
        left.update(&gpu.queue);
        right.update(&gpu.queue); // Must be a no-op upload, not a no-op draw.
        let after = draw(&mut left, &mut right);
        let pixel = |image: &headless_gpu::Image, x: u32| {
            let offset = ((8 * image.width + x) * 4) as usize;
            image.pixels[offset..offset + 4].to_vec()
        };
        assert_eq!(pixel(&before, 24), pixel(&after, 24));
        assert_ne!(pixel(&before, 8), pixel(&after, 8));
        after.assert_pixel_near(8, 8, [255, 255, 255, 255], 1);
    });
}
