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
    fixture_with_volume(gpu, vol)
}

fn fixture_with_volume(gpu: &Gpu, vol: CTVolume) -> (AppView, CTVolume) {
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
    let has_mip = app
        .layout
        .views()
        .iter()
        .any(|view| view.as_any().is::<MipView>());
    let has_mesh = app
        .layout
        .views()
        .iter()
        .any(|view| view.as_any().is::<MeshView>());
    let image = render_encoded_offscreen(gpu, 32, 16, FORMAT, |encoder, target| {
        executor
            .execute_frame(
                encoder,
                target,
                &mut pool,
                &gpu.device,
                32,
                16,
                has_mesh,
                has_mip,
                has_mpr,
                |context| {
                    assert!(matches!(
                        context.pass_id,
                        PassId::SlicePass | PassId::MipPass | PassId::MeshPass
                    ));
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
    app.set_pan(0, 0.1, -0.1).unwrap();
    app.set_window_level(0, 500.0).unwrap();
    app.set_window_width(0, 1000.0).unwrap();
    let state_before: Vec<_> = app
        .layout
        .views()
        .iter()
        .map(|view| {
            format!(
                "{:?}",
                view.as_any()
                    .downcast_ref::<MprView>()
                    .unwrap()
                    .save_state()
            )
        })
        .collect();
    for gpu in [&original, &replacement] {
        gpu.validated("Bulk MPR rebinding", || {
            let format = wgpu::TextureFormat::Bgra8Unorm;
            let encoding = VolumeEncoding::for_float_support(false);
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
            app.rebind_gpu_resources(
                ViewGpuConfiguration {
                    device: &gpu.device,
                    queue: &gpu.queue,
                    format,
                    use_float_volume_texture: false,
                },
                Some(content),
            )
            .unwrap();
            let expected = app.view_factory.mpr_render_context();
            assert!(!Arc::ptr_eq(&original_context, &expected));
            for (index, view) in app.layout.views_mut().iter_mut().enumerate() {
                let mpr = view.as_any_mut().downcast_mut::<MprView>().unwrap();
                assert!(Arc::ptr_eq(&expected, mpr.render_context()));
                assert!(mpr.needs_redraw());
                assert_eq!(format!("{:?}", mpr.save_state()), state_before[index]);
                let pixels = headless_gpu::render_offscreen(gpu, 32, 16, format, false, |pass| {
                    mpr.render(pass).unwrap()
                });
                let gray = if index == 0 { 128 } else { 145 };
                pixels.assert_pixel_near(8 + index as u32 * 16, 8, [gray, gray, gray, 255], 2);
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
            // An explicit compatible rebind retains the factory context and volume textures.
            let contents: Vec<_> = app
                .layout
                .views()
                .iter()
                .map(|view| {
                    Arc::clone(
                        view.as_any()
                            .downcast_ref::<MprView>()
                            .unwrap()
                            .render_content(),
                    )
                })
                .collect();
            app.rebind_gpu_resources(
                ViewGpuConfiguration {
                    device: &gpu.device,
                    queue: &gpu.queue,
                    format,
                    use_float_volume_texture: false,
                },
                None,
            )
            .unwrap();
            assert!(Arc::ptr_eq(
                &expected,
                &app.view_factory.mpr_render_context()
            ));
            for (index, view) in app.layout.views().iter().enumerate() {
                let mpr = view.as_any().downcast_ref::<MprView>().unwrap();
                assert!(Arc::ptr_eq(&expected, mpr.render_context()));
                assert!(Arc::ptr_eq(&contents[index], mpr.render_content()));
                assert!(mpr.needs_redraw());
                assert_eq!(format!("{:?}", mpr.save_state()), state_before[index]);
            }
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

#[test]
fn failed_second_view_gpu_preparation_preserves_all_existing_owners() {
    let original = Gpu::new();
    let replacement = Gpu::new();
    let (mut app, vol) = fixture(&original);
    let labels = Arc::new(
        RenderContent::from_labels_r8(
            &original.device,
            &original.queue,
            &[1; 8],
            "Labels",
            2,
            2,
            2,
        )
        .unwrap(),
    );
    let second = app.layout.views_mut()[1]
        .as_any_mut()
        .downcast_mut::<MprView>()
        .unwrap();
    second.set_segmentation(&original.device, Some(labels));
    // External GPU-only labels cannot silently disappear during a device swap.
    frame(&original, &mut app);
    let context = app.view_factory.mpr_render_context();
    let contents: Vec<_> = app
        .layout
        .views()
        .iter()
        .map(|v| {
            v.as_any()
                .downcast_ref::<MprView>()
                .unwrap()
                .render_content()
                .clone()
        })
        .collect();
    let encoding = VolumeEncoding::for_float_support(false);
    let next = Arc::new(
        RenderContent::from_bytes(
            &replacement.device,
            &replacement.queue,
            &encoding.encode_volume(vol.voxel_data()),
            "Replacement",
            2,
            2,
            2,
            encoding,
        )
        .unwrap(),
    );
    assert!(app
        .rebind_gpu_resources(
            ViewGpuConfiguration {
                device: &replacement.device,
                queue: &replacement.queue,
                format: wgpu::TextureFormat::Bgra8Unorm,
                use_float_volume_texture: false,
            },
            Some(next)
        )
        .is_err());
    assert!(Arc::ptr_eq(
        &context,
        &app.view_factory.mpr_render_context()
    ));
    assert!(!app.layout.needs_redraw());
    for (v, old) in app.layout.views().iter().zip(contents) {
        let mpr = v.as_any().downcast_ref::<MprView>().unwrap();
        assert!(Arc::ptr_eq(&old, mpr.render_content()));
    }
    original.validated("Retained old owners after failed GPU preparation", || {
        frame(&original, &mut app);
    });
}
#[test]
fn mip_primitive_changes_invalidate_and_noop_controls_remain_idle() {
    let gpu = Gpu::new();
    let (mut app, vol) = fixture(&gpu);
    let content = app.layout.views()[0]
        .as_any()
        .downcast_ref::<MprView>()
        .unwrap()
        .render_content()
        .clone();
    app.set_layout_mode_single(content, &vol, 1, 0).unwrap();
    frame(&gpu, &mut app);
    let mip = app.layout.views_mut()[0]
        .as_any_mut()
        .downcast_mut::<MipView>()
        .unwrap();
    mip.set_ball_enabled(true);
    assert!(mip.needs_redraw());
    frame(&gpu, &mut app);
    let mip = app.layout.views_mut()[0]
        .as_any_mut()
        .downcast_mut::<MipView>()
        .unwrap();
    mip.set_ball_enabled(true);
    assert!(!mip.needs_redraw());
    mip.set_ball(0, [0.5, 0.5, 0.5], 0.1);
    assert!(mip.needs_redraw());
}

mod transitions {
    use super::*;
    use crate::application::{
        app_model::AppModel,
        view_layout_transition::{
            self as transition, GpuPreparation, Preparation, TransitionRequest, ViewInput,
        },
    };
    use crate::core::error::{KeplerError, KeplerResult};
    use std::cell::Cell;

    fn request(mode: usize) -> TransitionRequest {
        TransitionRequest {
            mode,
            mesh_index: Some(3),
            orientation_slot: Some(1),
            mip_index: Some(2),
            orientation_index: 3,
        }
    }
    struct Failing<'a> {
        gpu: GpuPreparation<'a>,
        fail_at: usize,
        step: Cell<usize>,
    }
    impl Failing<'_> {
        fn check(&self) -> KeplerResult<()> {
            let step = self.step.get();
            self.step.set(step + 1);
            if step == self.fail_at {
                Err(KeplerError::Graphics(format!(
                    "Injected preparation failure {step}"
                )))
            } else {
                Ok(())
            }
        }
    }
    impl Preparation for Failing<'_> {
        fn upload(&self, model: &AppModel) -> KeplerResult<Arc<RenderContent>> {
            self.check()?;
            self.gpu.upload(model)
        }
        fn create_view(
            &self,
            factory: &DefaultViewFactory,
            content: Arc<RenderContent>,
            input: ViewInput<'_>,
        ) -> KeplerResult<Box<dyn View>> {
            self.check()?;
            self.gpu.create_view(factory, content, input)
        }
    }
    #[test]
    fn every_preparation_failure_preserves_release_layout_state_and_pending_frame() {
        let gpu = Gpu::new();
        gpu.validated("Transactional release layouts", || {
            for mode in [0, 1, 2, 3, 4] {
                let steps = if mode <= 2 { 2 } else { 7 };
                for fail_at in 0..steps {
                    let (mut app, vol) = fixture(&gpu);
                    let mut model = AppModel::new(false);
                    model.load_volume(vol.clone()).unwrap();
                    let mut orientations = [0, 1, 2, 0];
                    app.set_window_level(0, 321.0).unwrap();
                    app.set_scale(0, 1.5).unwrap();
                    let v = app.layout.views_mut()[0]
                        .as_any_mut()
                        .downcast_mut::<MprView>()
                        .unwrap();
                    v.set_ball_enabled(true);
                    v.set_ball(0, [0.4, 0.5, 0.6], 0.1);
                    app.save_view_states();
                    if fail_at % 2 == 0 {
                        frame(&gpu, &mut app);
                    }
                    let identities: Vec<_> = app
                        .layout
                        .views()
                        .iter()
                        .map(|v| v.as_ref() as *const dyn View as *const () as usize)
                        .collect();
                    let state = format!("{:?}", app.saved_states);
                    let dirty = app.layout.needs_redraw();
                    let strategy = app.layout.strategy_id().to_string();
                    let before = app.layout.views()[0]
                        .as_any()
                        .downcast_ref::<MprView>()
                        .unwrap()
                        .save_state()
                        .unwrap();
                    let prep = Failing {
                        gpu: GpuPreparation {
                            device: &gpu.device,
                            queue: &gpu.queue,
                        },
                        fail_at,
                        step: Cell::new(0),
                    };
                    assert!(transition::apply(
                        &model,
                        &mut app,
                        &mut orientations,
                        request(mode),
                        &prep
                    )
                    .is_err());
                    assert_eq!(orientations, [0, 1, 2, 0]);
                    assert_eq!(app.layout.strategy_id(), strategy);
                    assert_eq!(app.layout.needs_redraw(), dirty);
                    assert_eq!(format!("{:?}", app.saved_states), state);
                    assert_eq!(
                        app.layout
                            .views()
                            .iter()
                            .map(|v| v.as_ref() as *const dyn View as *const () as usize)
                            .collect::<Vec<_>>(),
                        identities
                    );
                    let after = app.layout.views()[0]
                        .as_any()
                        .downcast_ref::<MprView>()
                        .unwrap()
                        .save_state()
                        .unwrap();
                    assert_eq!(format!("{before:?}"), format!("{after:?}"));
                    assert_eq!(model.volume().unwrap().dimensions, vol.dimensions);
                }
            }
        });
    }
    #[test]
    fn release_modes_commit_only_complete_layouts_and_keep_public_adapters() {
        let gpu = Gpu::new();
        gpu.validated("Release mode layouts", || {
            let (mut app, vol) = fixture(&gpu);
            let mut model = AppModel::new(false);
            model.load_volume(vol.clone()).unwrap();
            let mut orientations = [0, 1, 2, 0];
            let prep = GpuPreparation {
                device: &gpu.device,
                queue: &gpu.queue,
            };
            for mode in [0, 1, 2, 3, 4] {
                transition::apply(&model, &mut app, &mut orientations, request(mode), &prep)
                    .unwrap();
                assert_eq!(orientations, [0, 3, 2, 0]);
                assert_eq!(app.layout.views().len(), if mode <= 2 { 1 } else { 4 });
                assert_eq!(
                    app.layout.strategy_id(),
                    if mode <= 2 {
                        "OneCellLayout"
                    } else if mode == 4 {
                        "LargeLeft3RightLayout"
                    } else {
                        "GridLayout"
                    }
                );
                assert!(app.layout.needs_redraw());
                frame(&gpu, &mut app);
                assert!(!app.layout.needs_redraw());
            }
            let content = prep.upload(&model).unwrap();
            let before = app.layout.views().len();
            assert!(app
                .configure_mesh_layout(content.clone(), &vol, [0, 1, 2, 4], None, None)
                .is_err());
            assert_eq!(app.layout.views().len(), before);
            assert!(app
                .set_layout_mode_single(content.clone(), &vol, 0, 4)
                .is_err());
            assert_eq!(app.layout.views().len(), before);
            app.set_layout_three(content, &vol, [0, 1, 2, 0], None, None)
                .unwrap();
            assert_eq!(app.layout.strategy_id(), "LargeLeft3RightLayout");
        });
    }
    #[test]
    fn missing_volume_and_invalid_selections_leave_layout_untouched() {
        let gpu = Gpu::new();
        let (mut app, vol) = fixture(&gpu);
        let mut model = AppModel::new(false);
        let mut orientations = [0, 1, 2, 0];
        let prep = GpuPreparation {
            device: &gpu.device,
            queue: &gpu.queue,
        };
        assert!(transition::apply(&model, &mut app, &mut orientations, request(3), &prep).is_err());
        model.load_volume(vol).unwrap();
        for selection in 0..4 {
            let mut req = request(3);
            match selection {
                0 => req.orientation_slot = Some(4),
                1 => req.orientation_index = 4,
                2 => req.mip_index = Some(4),
                _ => req.mesh_index = Some(4),
            }
            assert!(transition::apply(&model, &mut app, &mut orientations, req, &prep).is_err());
            assert_eq!(app.layout.views().len(), 2);
            assert_eq!(orientations, [0, 1, 2, 0]);
        }
    }
}
