//! CPU-only layout invalidation tests. Surface scheduling is tested in its own module.
use kepler_wgpu::rendering::view::{DynamicLayout, GridLayout, LayoutContainer};
use kepler_wgpu::{Renderable, View};
use std::any::Any;

struct StaticView;
impl Renderable for StaticView {
    fn update(&mut self, _: &wgpu::Queue) {}
    fn render(&mut self, _: &mut wgpu::RenderPass) -> kepler_wgpu::core::error::KeplerResult<()> {
        Ok(())
    }
}
impl View for StaticView {
    fn needs_redraw(&self) -> bool {
        false
    }
    fn position(&self) -> (i32, i32) {
        (0, 0)
    }
    fn dimensions(&self) -> (u32, u32) {
        (16, 16)
    }
    fn move_to(&mut self, _: (i32, i32)) {}
    fn resize(&mut self, _: (u32, u32)) {}
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

fn layout() -> DynamicLayout {
    DynamicLayout::new(
        (32, 32),
        Box::new(GridLayout {
            rows: 2,
            cols: 2,
            spacing: 0,
        }),
    )
}

#[test]
fn downstream_render_signatures_keep_unit_success_and_separate_pass_lifetimes() {
    use kepler_wgpu::core::error::{KeplerError, KeplerResult, SurfaceFailure, SurfaceFailureKind};
    use kepler_wgpu::rendering::core::PassContext;
    fn is_timeout(error: &KeplerError) -> bool {
        matches!(error, KeplerError::Surface(failure) if failure.kind == SurfaceFailureKind::Timeout)
    }
    let owned: KeplerError =
        SurfaceFailure::new(SurfaceFailureKind::Timeout, "acquire", "timeout").into();
    assert!(is_timeout(&owned));
    assert!(!is_timeout(&KeplerError::Graphics("failed pass".into())));
    let _: fn(&mut kepler_wgpu::App) -> KeplerResult<()> = kepler_wgpu::App::render;
    fn draw(view: &mut dyn Renderable, context: PassContext<'_, '_>) -> KeplerResult<()> {
        view.render(context.pass)
    }
    let _custom_renderer = draw;
    let _view: Box<dyn Renderable> = Box::new(StaticView);
}

#[test]
fn layout_creation_clearing_and_replacement_require_presentation() {
    let mut layout = layout();
    assert!(layout.needs_redraw());
    layout.mark_presented();
    assert!(!layout.needs_redraw());
    layout.add_view(Box::new(StaticView));
    assert!(layout.needs_redraw());
    layout.mark_presented();
    assert!(!layout.needs_redraw());
    assert!(layout.replace_view_at(0, Box::new(StaticView)).is_some());
    assert!(layout.needs_redraw());
    layout.mark_presented();
    layout.remove_all();
    assert!(
        layout.needs_redraw(),
        "removing the final view must clear stale pixels"
    );
    layout.mark_presented();
    assert!(!layout.needs_redraw());
    layout.remove_all();
    assert!(
        !layout.needs_redraw(),
        "clearing an already empty layout is a no-op"
    );
}

#[test]
fn minimized_layout_can_accept_pending_views_without_underflow() {
    let mut layout = DynamicLayout::new(
        (0, 0),
        Box::new(GridLayout {
            rows: 2,
            cols: 2,
            spacing: 2,
        }),
    );
    layout.add_view(Box::new(StaticView));
    assert!(layout.needs_redraw());
    layout.resize((1, 1));
    assert!(layout.needs_redraw());
}

#[test]
fn layout_noop_resize_and_getters_do_not_request_frames() {
    let mut layout = layout();
    layout.add_view(Box::new(StaticView));
    layout.mark_presented();
    let _ = layout.get_view_by_index(0);
    let _ = layout.views();
    let _ = layout.get_view_mut(0);
    layout.resize((32, 32));
    assert!(!layout.needs_redraw());
    layout.resize((64, 64));
    assert!(layout.needs_redraw());
    layout.mark_presented();
    layout.set_strategy(Box::new(GridLayout {
        rows: 1,
        cols: 1,
        spacing: 0,
    }));
    assert!(layout.needs_redraw());
}
