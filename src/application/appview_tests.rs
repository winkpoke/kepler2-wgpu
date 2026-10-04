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

mod transitions {
    use super::*;
    use crate::application::app_model::AppModel;
    use crate::application::view_layout_transition::{
        self as transition, GpuPreparation, Preparation, TransitionRequest, ViewInput,
    };
    use crate::core::error::{KeplerError, KeplerResult};
    use std::cell::{Cell, RefCell};

    fn request(mode: usize) -> TransitionRequest {
        TransitionRequest {
            mode,
            reuse_mesh: true,
            world_min: None,
            world_max: None,
            mesh_index: None,
            orientation_slot: None,
            iso_range: (200.0, 800.0),
            mip_index: None,
            orientation_index: 0,
        }
    }

    struct FailurePreparation<'a> {
        gpu: GpuPreparation<'a>,
        fail_upload: bool,
        fail_view: usize,
        calls: Cell<usize>,
        candidates: RefCell<Vec<std::sync::Weak<RenderContent>>>,
    }

    impl<'a> FailurePreparation<'a> {
        fn new(gpu: &'a Gpu, fail_upload: bool, fail_view: usize) -> Self {
            Self {
                gpu: GpuPreparation {
                    device: &gpu.device,
                    queue: &gpu.queue,
                },
                fail_upload,
                fail_view,
                calls: Cell::new(0),
                candidates: RefCell::new(Vec::new()),
            }
        }
    }

    impl Preparation for FailurePreparation<'_> {
        fn upload(&self, model: &AppModel) -> KeplerResult<Arc<RenderContent>> {
            if self.fail_upload {
                return Err(KeplerError::Graphics("injected upload error".into()));
            }
            self.gpu.upload(model)
        }

        fn create_view(
            &self,
            factory: &DefaultViewFactory,
            content: Arc<RenderContent>,
            input: ViewInput<'_>,
        ) -> KeplerResult<Box<dyn View>> {
            self.candidates.borrow_mut().push(Arc::downgrade(&content));
            let call = self.calls.get() + 1;
            self.calls.set(call);
            if call == self.fail_view {
                return Err(KeplerError::Graphics("injected constructor error".into()));
            }
            self.gpu.create_view(factory, content, input)
        }
    }

    #[derive(Debug, PartialEq)]
    struct ViewSnapshot {
        identity: usize,
        position: (i32, i32),
        dimensions: (u32, u32),
        interaction: String,
        pending: bool,
    }

    #[derive(Debug, PartialEq)]
    struct Snapshot {
        views: Vec<ViewSnapshot>,
        strategy: String,
        pending: bool,
        captured: String,
        orientations: [usize; 4],
        mesh_vertices: Option<Vec<u8>>,
        mesh_indices: Option<Vec<u32>>,
        mesh_identity: Option<usize>,
        enable_mesh: bool,
        volume_identity: Option<usize>,
        voxel_identity: Option<usize>,
        voxels: Option<Vec<i16>>,
    }

    fn snapshot(
        app: &AppView,
        model: &AppModel,
        mesh: &Option<Mesh>,
        orientations: [usize; 4],
    ) -> Snapshot {
        Snapshot {
            views: app
                .layout
                .views()
                .iter()
                .map(|view| {
                    let interaction = if let Some(mpr) = view.as_any().downcast_ref::<MprView>() {
                        format!(
                            "{:?} {:?} {:?}",
                            mpr.get_orientation(),
                            mpr.save_state(),
                            mpr.get_base()
                        )
                    } else if let Some(mip) = view.as_any().downcast_ref::<MipView>() {
                        format!(
                            "{:?} {:?} {:?} {:?} {:?}",
                            mip.config(),
                            mip.get_scale(),
                            mip.get_pan(),
                            mip.get_window_level(),
                            mip.get_rotation_radians()
                        )
                    } else if let Some(mesh) = view.as_any().downcast_ref::<MeshView>() {
                        format!(
                            "{:?} {:?} {:?} {:?}",
                            mesh.get_rotation(),
                            mesh.get_scale_factor(),
                            mesh.get_pan(),
                            mesh.get_opacity()
                        )
                    } else {
                        panic!("unexpected fixture view")
                    };
                    ViewSnapshot {
                        identity: view.as_ref() as *const dyn View as *const () as usize,
                        position: view.position(),
                        dimensions: view.dimensions(),
                        interaction,
                        pending: view.needs_redraw(),
                    }
                })
                .collect(),
            strategy: app.layout.strategy_id().into(),
            pending: app.layout.needs_redraw(),
            captured: format!("{:?}", app.saved_states),
            orientations,
            mesh_vertices: mesh
                .as_ref()
                .map(|mesh| bytemuck::cast_slice(&mesh.vertices).to_vec()),
            mesh_indices: mesh.as_ref().map(|mesh| mesh.indices.clone()),
            mesh_identity: mesh.as_ref().map(|mesh| mesh.vertices.as_ptr() as usize),
            enable_mesh: model.enable_mesh,
            volume_identity: model
                .volume()
                .ok()
                .map(|vol| vol as *const CTVolume as usize),
            voxel_identity: model
                .volume()
                .ok()
                .map(|vol| vol.voxel_data().as_ptr() as usize),
            voxels: model.volume().ok().map(|vol| vol.voxel_data().to_vec()),
        }
    }

    #[test]
    fn returned_failures_preserve_all_committed_state_and_pixels() {
        let gpu = Gpu::new();
        gpu.validated("transactional transition returned errors", || {
            for fail_upload in [true, false] {
                for pending in [false, true] {
                    // Mesh extraction's existing Gaussian kernel requires more than two samples.
                    let (mut app, vol) = fixture_with_volume(
                        &gpu,
                        CTVolume::new((8, 8, 8), (1.0, 1.0, 1.0), vec![500; 512], Base::default()),
                    );
                    let mut model = AppModel::new(false);
                    model.load_volume(vol).unwrap();
                    let mut mesh = Some(Mesh::unit_cube());
                    let mut orientations = [0, 1, 2, 0];
                    app.set_scale(0, 1.3).unwrap();
                    app.set_pan(0, 0.1, -0.1).unwrap();
                    app.set_slice_mm(0, 0.5).unwrap();
                    app.set_window_level(0, 300.0).unwrap();
                    app.set_window_width(0, 900.0).unwrap();
                    app.layout.views_mut()[0]
                        .as_any_mut()
                        .downcast_mut::<MprView>()
                        .unwrap()
                        .set_oblique_rotation_radians(0.1, 0.2, 0.3)
                        .unwrap();
                    app.save_view_states();
                    let pixels = frame(&gpu, &mut app).pixels;
                    if pending {
                        app.set_scale(0, 1.4).unwrap();
                    }
                    let before = snapshot(&app, &model, &mesh, orientations);
                    let mut change = request(if fail_upload { 2 } else { 9 });
                    change.mesh_index = Some(3);
                    change.mip_index = Some(2);
                    change.orientation_slot = Some(0);
                    change.orientation_index = 3;
                    change.reuse_mesh = false;
                    let preparation = FailurePreparation::new(&gpu, fail_upload, 6);
                    let error = transition::apply(
                        &mut model,
                        &mut app,
                        &mut mesh,
                        &mut orientations,
                        change,
                        &preparation,
                    )
                    .unwrap_err();
                    assert!(error.to_string().contains(if fail_upload {
                        "upload preparation"
                    } else {
                        "create mesh view"
                    }));
                    let after = snapshot(&app, &model, &mesh, orientations);
                    assert_eq!(
                        after.views, before.views,
                        "displayed views changed after failure"
                    );
                    assert_eq!(after, before);
                    assert_eq!(preparation.calls.get(), if fail_upload { 0 } else { 6 });
                    assert!(preparation
                        .candidates
                        .borrow()
                        .iter()
                        .all(|content| content.upgrade().is_none()));
                    let after = frame(&gpu, &mut app);
                    if !pending {
                        assert_eq!(after.pixels, pixels);
                    }
                    assert!(!app.layout.needs_redraw());
                }
            }
        });
    }

    #[test]
    fn successful_modes_commit_defaults_orientations_cache_and_pending_frame() {
        let gpu = Gpu::new();
        gpu.validated("production mode cycle", || {
            let (mut app, vol) = fixture(&gpu);
            let mut model = AppModel::new(false);
            model.load_volume(vol).unwrap();
            let mut mesh = Some(Mesh::unit_cube());
            let original_mesh = mesh.as_ref().unwrap().vertices.as_ptr();
            let mut orientations = [0, 1, 2, 0];
            let preparation = GpuPreparation {
                device: &gpu.device,
                queue: &gpu.queue,
            };
            app.set_scale(0, 1.7).unwrap();
            app.set_pan(0, 0.2, -0.2).unwrap();
            app.set_slice_mm(0, 0.5).unwrap();
            app.set_window_level(0, 100.0).unwrap();
            app.layout.views_mut()[0]
                .as_any_mut()
                .downcast_mut::<MprView>()
                .unwrap()
                .set_oblique_rotation_radians(0.2, 0.3, 0.4)
                .unwrap();
            app.save_view_states();
            frame(&gpu, &mut app);
            let source_identity = model.volume().unwrap().voxel_data().as_ptr();
            for mode in [0, 1, 2, 99, 0] {
                let mut change = request(mode);
                if mode == 99 {
                    change.mesh_index = Some(3);
                    change.mip_index = Some(2);
                    change.orientation_slot = Some(1);
                    change.orientation_index = 3;
                }
                transition::apply(
                    &mut model,
                    &mut app,
                    &mut mesh,
                    &mut orientations,
                    change,
                    &preparation,
                )
                .unwrap();
                assert!(app.layout.needs_redraw());
                assert_eq!(
                    model.volume().unwrap().voxel_data().as_ptr(),
                    source_identity
                );
                assert_eq!(mesh.as_ref().unwrap().vertices.as_ptr(), original_mesh);
                if mode == 99 {
                    assert_eq!(app.layout.view_count(), 4);
                    assert_eq!(app.layout.strategy_id(), "GridLayout");
                    assert_eq!(orientations, [0, 3, 2, 0]);
                    assert_eq!(
                        *app.layout.views()[1]
                            .as_any()
                            .downcast_ref::<MprView>()
                            .unwrap()
                            .get_orientation(),
                        Orientation::Oblique
                    );
                    assert!(app.layout.views()[2].as_any().is::<MipView>());
                    assert!(app.layout.views()[3].as_any().is::<MeshView>());
                    assert_eq!(app.layout.views()[3].position(), (17, 9));
                    assert_eq!(app.layout.views()[3].dimensions(), (15, 7));
                    assert!(model.enable_mesh);
                } else {
                    assert_eq!(app.layout.view_count(), 1);
                    assert_eq!(app.layout.strategy_id(), "OneCellLayout");
                    let view = &app.layout.views()[0];
                    assert_eq!(view.position(), (0, 0));
                    assert_eq!(view.dimensions(), (32, 16));
                    match mode {
                        0 => {
                            let mpr = view.as_any().downcast_ref::<MprView>().unwrap();
                            assert_eq!(mpr.get_scale(), 1.0);
                            assert_eq!(mpr.get_translate_in_screen_coord(), [0.0; 3]);
                            assert_eq!(mpr.get_slice_mm(), 0.0);
                            assert_eq!(mpr.get_window_level(), 400.0);
                            assert_eq!(mpr.get_window_width(), 1500.0);
                            assert_eq!(mpr.get_oblique_rotation(), glam::Quat::IDENTITY);
                        }
                        1 => assert!(view.as_any().is::<MipView>()),
                        2 => assert!(view.as_any().is::<MeshView>()),
                        _ => unreachable!(),
                    }
                }
                frame(&gpu, &mut app);
                assert!(!app.layout.needs_redraw());
            }
            // Captured settings remain available for explicit restoration, never applied automatically.
            assert!(!app.saved_states.is_empty());
        });
    }

    #[test]
    fn malformed_upload_and_invalid_prerequisites_leave_old_layout_intact() {
        let gpu = Gpu::new();
        gpu.validated("transition prerequisite validation", || {
            let (mut app, vol) = fixture(&gpu);
            frame(&gpu, &mut app);
            let mut model = AppModel::new(false);
            let mut mesh = None;
            let mut orientations = [0, 1, 2, 0];
            let preparation = FailurePreparation::new(&gpu, false, usize::MAX);
            let before = snapshot(&app, &model, &mesh, orientations);
            let error = transition::apply(
                &mut model,
                &mut app,
                &mut mesh,
                &mut orientations,
                request(2),
                &preparation,
            )
            .unwrap_err();
            assert!(error.to_string().contains("loaded volume"));
            assert_eq!(snapshot(&app, &model, &mesh, orientations), before);
            model.load_volume(vol).unwrap();
            // Loading data itself does not replay the rejected request.
            assert_eq!(app.layout.view_count(), 2);
            for selection in 0..5 {
                let mut change = request(8);
                match selection {
                    0 => change.orientation_slot = Some(4),
                    1 => {
                        change.mode = 0;
                        change.orientation_index = 4;
                    }
                    2 => change.mip_index = Some(4),
                    3 => change.mesh_index = Some(4),
                    4 => change.mode = 2,
                    _ => unreachable!(),
                }
                let before = snapshot(&app, &model, &mesh, orientations);
                assert!(transition::apply(
                    &mut model,
                    &mut app,
                    &mut mesh,
                    &mut orientations,
                    change,
                    &preparation
                )
                .is_err());
                assert_eq!(snapshot(&app, &model, &mesh, orientations), before);
            }
            assert_eq!(preparation.calls.get(), 0);
            model
                .load_volume(CTVolume::new(
                    (2, 2, 2),
                    (1.0, 1.0, 1.0),
                    vec![500; 7],
                    Base::default(),
                ))
                .unwrap();
            let before = snapshot(&app, &model, &mesh, orientations);
            let error = transition::apply(
                &mut model,
                &mut app,
                &mut mesh,
                &mut orientations,
                request(1),
                &preparation,
            )
            .unwrap_err();
            assert!(error
                .to_string()
                .contains("upload preparation for dimensions (2, 2, 2)"));
            assert_eq!(snapshot(&app, &model, &mesh, orientations), before);
            assert_eq!(preparation.calls.get(), 0);
            // Old uploaded content still renders even though the model contains malformed replacement data.
            frame(&gpu, &mut app).assert_pixel_near(8, 8, [145, 145, 145, 255], 2);
        });
    }

    struct OffsetLayout;
    impl crate::rendering::view::LayoutStrategy for OffsetLayout {
        fn id(&self) -> &str {
            "OffsetLayout"
        }
        fn calculate_position_and_size(
            &self,
            index: u32,
            count: u32,
            dim: (u32, u32),
        ) -> ((i32, i32), (u32, u32)) {
            ((index as i32 * 8, 2), (dim.0 / count, dim.1 - 2))
        }
    }

    #[test]
    fn mixed_precedence_custom_strategy_and_mesh_generation_retain_legacy_policy() {
        let gpu = Gpu::new();
        gpu.validated("legacy transition compatibility", || {
            let (mut app, vol) = fixture_with_volume(
                &gpu,
                CTVolume::new((8, 8, 8), (1.0, 1.0, 1.0), vec![500; 512], Base::default()),
            );
            let mut model = AppModel::new(false);
            model.load_volume(vol).unwrap();
            let mut mesh = Some(Mesh::unit_cube());
            let mut orientations = [0, 1, 2, 0];
            let preparation = GpuPreparation {
                device: &gpu.device,
                queue: &gpu.queue,
            };
            app.layout.set_strategy(Box::new(OffsetLayout));
            let mut change = request(usize::MAX);
            change.mesh_index = Some(2);
            change.mip_index = Some(2);
            change.orientation_slot = Some(3);
            change.orientation_index = 3;
            let cache_identity = mesh.as_ref().unwrap().vertices.as_ptr();
            transition::apply(
                &mut model,
                &mut app,
                &mut mesh,
                &mut orientations,
                change,
                &preparation,
            )
            .unwrap();
            assert_eq!(app.layout.strategy_id(), "OffsetLayout");
            assert_eq!(app.layout.views()[2].position(), (16, 2));
            assert_eq!(app.layout.views()[2].dimensions(), (8, 14));
            assert!(app.layout.views()[2].as_any().is::<MeshView>());
            assert!(app.layout.views()[3].as_any().is::<MprView>());
            assert_eq!(
                *app.layout.views()[3]
                    .as_any()
                    .downcast_ref::<MprView>()
                    .unwrap()
                    .get_orientation(),
                Orientation::Oblique
            );
            assert_eq!(mesh.as_ref().unwrap().vertices.as_ptr(), cache_identity);

            // Unused MIP orientation/slot arguments remain unrestricted in single MIP mode.
            let mut change = request(1);
            change.orientation_index = usize::MAX;
            change.mip_index = Some(usize::MAX);
            transition::apply(
                &mut model,
                &mut app,
                &mut mesh,
                &mut orientations,
                change,
                &preparation,
            )
            .unwrap();
            assert!(app.is_one_cell_layout());

            // A mesh request still generates and caches content even in single MPR mode.
            let mut change = request(0);
            change.mesh_index = Some(usize::MAX);
            change.reuse_mesh = false;
            change.iso_range = (1000.0, 2000.0);
            transition::apply(
                &mut model,
                &mut app,
                &mut mesh,
                &mut orientations,
                change,
                &preparation,
            )
            .unwrap();
            assert!(app.layout.views()[0].as_any().is::<MprView>());
            assert_ne!(mesh.as_ref().unwrap().vertices.as_ptr(), cache_identity);
            assert_eq!(mesh.as_ref().unwrap().vertices.len(), 3);
            assert_eq!(mesh.as_ref().unwrap().indices, vec![0, 1, 2]);
            assert!(model.enable_mesh);

            // Generation is also triggered by an empty cache despite reuse=true.
            mesh = None;
            let mut change = request(2);
            change.mesh_index = Some(0);
            transition::apply(
                &mut model,
                &mut app,
                &mut mesh,
                &mut orientations,
                change,
                &preparation,
            )
            .unwrap();
            assert!(mesh.is_some());
            assert!(app.layout.views()[0].as_any().is::<MeshView>());
        });
    }

    #[test]
    fn failures_from_single_mip_or_mesh_preserve_settings_and_pending_work() {
        let gpu = Gpu::new();
        gpu.validated("failed transitions from MIP and mesh", || {
            for mode in [1, 2] {
                for pending in [false, true] {
                    let (mut app, vol) = fixture(&gpu);
                    let mut model = AppModel::new(false);
                    model.load_volume(vol).unwrap();
                    let mut mesh = Some(Mesh::unit_cube());
                    let mut orientations = [0, 1, 2, 0];
                    let gpu_preparation = GpuPreparation {
                        device: &gpu.device,
                        queue: &gpu.queue,
                    };
                    transition::apply(
                        &mut model,
                        &mut app,
                        &mut mesh,
                        &mut orientations,
                        request(mode),
                        &gpu_preparation,
                    )
                    .unwrap();
                    if let Some(mip) = app.layout.views_mut()[0]
                        .as_any_mut()
                        .downcast_mut::<MipView>()
                    {
                        mip.set_scale(1.4);
                        mip.set_pan(0.1, 0.2);
                        mip.set_window_level(120.0).unwrap();
                        mip.set_window_width(800.0).unwrap();
                        mip.set_rotation_radians(0.1, 0.2, 0.3);
                    } else {
                        let mesh = app.layout.views_mut()[0]
                            .as_any_mut()
                            .downcast_mut::<MeshView>()
                            .unwrap();
                        mesh.set_scale_factor(1.4);
                        mesh.set_pan(0.1, 0.2);
                        mesh.set_rotation(glam::Mat4::from_rotation_y(0.3));
                        mesh.set_opacity(0.6);
                    }
                    frame(&gpu, &mut app);
                    if pending {
                        if let Some(mip) = app.layout.views_mut()[0]
                            .as_any_mut()
                            .downcast_mut::<MipView>()
                        {
                            mip.set_scale(1.6);
                        } else {
                            app.layout.views_mut()[0]
                                .as_any_mut()
                                .downcast_mut::<MeshView>()
                                .unwrap()
                                .set_scale_factor(1.6);
                        }
                    }
                    for fail_upload in [true, false] {
                        let before = snapshot(&app, &model, &mesh, orientations);
                        let failure = FailurePreparation::new(&gpu, fail_upload, 3);
                        assert!(transition::apply(
                            &mut model,
                            &mut app,
                            &mut mesh,
                            &mut orientations,
                            request(9),
                            &failure
                        )
                        .is_err());
                        assert_eq!(snapshot(&app, &model, &mesh, orientations), before);
                    }
                    frame(&gpu, &mut app);
                    assert!(!app.layout.needs_redraw());
                }
            }
        });
    }
}
