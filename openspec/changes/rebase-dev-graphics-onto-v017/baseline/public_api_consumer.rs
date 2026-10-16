//! Compile-only external consumer frozen against v0.17.0; no hardware or window startup.
#![allow(dead_code)]
use kepler_wgpu::{GLCanvas, Renderable, View};
use kepler_wgpu::core::WindowLevel;
use kepler_wgpu::core::error::KeplerError;
use kepler_wgpu::data::volume_encoding::VolumeEncoding;
use kepler_wgpu::rendering::core::render_pass::PassContext;
use kepler_wgpu::rendering::view::{MprRenderContext, mesh::MultiMeshContext};

kepler_wgpu::define_dicom_struct!(ExternalDicom, {
    (patient_name, String, "PatientName", false),
    (patient_id, String, "PatientID", true),
});

struct CustomView;
impl Renderable for CustomView {
    fn update(&mut self, _: &wgpu::Queue) {}
    fn render(&mut self, _: &mut wgpu::RenderPass) -> Result<(), wgpu::SurfaceError> { Ok(()) }
}
impl View for CustomView {
    fn position(&self) -> (i32, i32) { (0, 0) }
    fn dimensions(&self) -> (u32, u32) { (1, 1) }
    fn move_to(&mut self, _: (i32, i32)) {}
    fn resize(&mut self, _: (u32, u32)) {}
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
}

fn release_consumers() {
    let _ = WindowLevel { window_width: 10_000.0, window_level: 5_000.0, bias: 0.0, dirty: true };
    let _: fn(&GLCanvas, usize, Option<usize>, Option<usize>, Option<usize>, usize) = GLCanvas::set_render_mode;
    let _: fn(&wgpu::Device, &wgpu::Queue) -> MprRenderContext = MprRenderContext::new;
    let _: fn(&wgpu::Device, &wgpu::Queue) -> MultiMeshContext = MultiMeshContext::new;
    let _: KeplerError = wgpu::SurfaceError::Lost.into();
    let _ = ExternalDicom::new(String::new(), None).format_tags();
    match (VolumeEncoding::HuPackedRg8 { offset: 1100.0 }) {
        VolumeEncoding::HuPackedRg8 { offset } => assert_eq!(offset, 1100.0),
        VolumeEncoding::HuFloat => {},
    }
}
fn published_pass_annotation<'a>(context: PassContext<'a>) { let _ = context.pass_id; }

#[cfg(target_arch = "wasm32")]
async fn browser_results(canvas: &GLCanvas) {
    let _: Result<String, String> = canvas.get_rotation(0).await;
    let _: Result<String, String> = canvas.export_current_obj().await;
    let _: Result<js_sys::Array, String> = canvas.export_split_obj().await;
    let _: Result<Box<[f32]>, String> = canvas.get_window_level(0).await;
}

// Exported sender macro still expands for an external Rust consumer.
#[cfg(not(target_arch = "wasm32"))]
mod sender_macro {
    use kepler_wgpu::application::gl_canvas::UserEvent;
    struct GLCanvas { proxy: winit::event_loop::EventLoopProxy<UserEvent> }
    kepler_wgpu::impl_user_event_senders_for_glcanvas!(set_level => SetWindowLevel(value: f32));
}
fn main() { release_consumers(); }
