use super::*;
use crate::core::coord::Base;
use crate::rendering::core::{PassExecutor, PassId};
use crate::rendering::view::mesh::mesh_texture_pool::MeshTexturePool;
use crate::Renderable;
use crate::VolumeEncoding;

#[path = "../../tests/common/headless_gpu.rs"]
mod headless_gpu;
use headless_gpu::{render_encoded_offscreen, Gpu, Image};

const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;

fn fixture(gpu: &Gpu) -> (AppView, CTVolume) {
    let vol = CTVolume::new((2, 2, 2), (1.0, 1.0, 1.0), vec![500; 8], Base::default());
    let factory = DefaultViewFactory::new(gpu.device.clone(), gpu.queue.clone(), FORMAT, false);
    let mut layout = DynamicLayout::new(
        (32, 16),
        Box::new(GridLayout {
            rows: 1,
            cols: 2,
            spacing: 0,
        }),
    );
    let first = factory
        .create_mpr_view(&vol, Orientation::Transverse, (0, 0), (16, 16))
        .unwrap();
    let content = first
        .as_any()
        .downcast_ref::<MprView>()
        .unwrap()
        .render_content()
        .clone();
    layout.add_view(first);
    layout.add_view(
        factory
            .create_mpr_view_with_content(content, &vol, Orientation::Transverse, (16, 0), (16, 16))
            .unwrap(),
    );
    (AppView::new(layout, factory), vol)
}

fn frame(gpu: &Gpu, app: &mut AppView) -> Image {
    let mut pool = MeshTexturePool::new();
    let mut executor = PassExecutor::new(FORMAT);
    app.layout.update(&gpu.queue);
    let has_mpr = app
        .layout
        .views()
        .iter()
        .any(|view| view.as_any().is::<MprView>());
    let image = render_encoded_offscreen(gpu, 32, 16, FORMAT, |encoder, target| {
        executor
            .execute_frame(
                encoder,
                target,
                &mut pool,
                &gpu.device,
                32,
                16,
                false,
                false,
                has_mpr,
                |context| {
                    assert_eq!(context.pass_id, PassId::SlicePass);
                    app.render_pass(context)
                        .map_err(|error| Box::new(error) as Box<dyn std::error::Error>)
                },
            )
            .unwrap();
    });
    app.layout.mark_presented();
    image
}

#[test]
fn production_frames_redraw_clean_views_and_clear_removed_layouts() {
    let gpu = Gpu::new();
    gpu.validated("AppView complete-frame presentation", || {
        let (mut app, _) = fixture(&gpu);
        let before = frame(&gpu, &mut app);
        assert!(!app.layout.needs_redraw());
        app.set_window_level(0, -1000.0).unwrap();
        assert!(app.layout.needs_redraw());
        let after = frame(&gpu, &mut app);
        let pixel = |image: &Image, x: u32| {
            let offset = ((8 * image.width + x) * 4) as usize;
            <[u8; 4]>::try_from(&image.pixels[offset..offset + 4]).unwrap()
        };
        assert_eq!(pixel(&before, 24), pixel(&after, 24));
        assert_ne!(pixel(&before, 8), pixel(&after, 8));
        after.assert_pixel_near(8, 8, [255, 255, 255, 255], 1);
        app.layout.remove_all();
        assert!(app.layout.needs_redraw());
        let empty = frame(&gpu, &mut app);
        empty.assert_pixel_near(8, 8, [26, 26, 26, 255], 1);
        empty.assert_pixel_near(24, 8, [26, 26, 26, 255], 1);
        assert!(!app.layout.needs_redraw());
    });
}

#[test]
fn bulk_rebinding_and_subsequent_creation_share_the_replacement_factory_context() {
    let original = Gpu::new();
    let replacement = Gpu::new();
    let (mut app, vol) = fixture(&original);
    let original_context = app.view_factory.mpr_render_context();
    app.set_scale(0, 1.2).unwrap();
    for gpu in [&original, &replacement] {
        gpu.validated("Bulk MPR rebinding", || {
            let format = wgpu::TextureFormat::Bgra8Unorm;
            let encoding = VolumeEncoding::PackedI16Rg8;
            let content = Arc::new(
                RenderContent::from_bytes(
                    &gpu.device,
                    &gpu.queue,
                    &encoding.encode_volume(vol.voxel_data()),
                    "Replacement volume",
                    2,
                    2,
                    2,
                    encoding,
                )
                .unwrap(),
            );
            app.view_factory =
                DefaultViewFactory::new(gpu.device.clone(), gpu.queue.clone(), format, false);
            app.rebuild_gpu_resources(&gpu.device, &gpu.queue, format, Some(content));
            let expected = app.view_factory.mpr_render_context();
            assert!(!Arc::ptr_eq(&original_context, &expected));
            for view in app.layout.views_mut() {
                let mpr = view.as_any_mut().downcast_mut::<MprView>().unwrap();
                assert!(Arc::ptr_eq(&expected, mpr.render_context()));
                assert!(mpr.needs_redraw());
                headless_gpu::render_offscreen(gpu, 32, 16, format, false, |pass| {
                    mpr.render(pass).unwrap()
                });
            }
            assert_eq!(
                app.layout.views()[0]
                    .as_any()
                    .downcast_ref::<MprView>()
                    .unwrap()
                    .get_scale(),
                1.2
            );
            let next = app
                .view_factory
                .create_mpr_view(&vol, Orientation::Sagittal, (0, 0), (16, 16))
                .unwrap();
            assert!(Arc::ptr_eq(
                &expected,
                next.as_any()
                    .downcast_ref::<MprView>()
                    .unwrap()
                    .render_context()
            ));
        });
    }
}

#[test]
fn control_and_linked_center_updates_invalidate_only_changed_views() {
    let gpu = Gpu::new();
    gpu.validated("AppView control invalidation", || {
        let (mut app, _) = fixture(&gpu);
        frame(&gpu, &mut app);
        app.set_scale(0, 1.0).unwrap();
        assert!(!app.layout.needs_redraw());
        app.set_slice_mm(0, 0.5).unwrap();
        assert!(app.layout.views()[0].needs_redraw());
        assert!(!app.layout.views()[1].needs_redraw());
        let point = app.layout.views()[0]
            .as_any()
            .downcast_ref::<MprView>()
            .unwrap()
            .screen_coord_to_world([0.25, 0.25, 0.0]);
        app.set_center_at_point_in_mm(1, point).unwrap();
        assert!(app.layout.views()[1].needs_redraw());
        frame(&gpu, &mut app);
        assert!(!app.layout.needs_redraw());
    });
}
