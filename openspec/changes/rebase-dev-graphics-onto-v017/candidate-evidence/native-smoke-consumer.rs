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
    fn render(&mut self, _: &mut wgpu::RenderPass) -> kepler_wgpu::core::error::KeplerResult<()> { Ok(()) }
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
    let _: KeplerError = kepler_wgpu::core::error::SurfaceFailure::new(kepler_wgpu::core::error::SurfaceFailureKind::Lost, "probe", "lost").into();
    let _ = ExternalDicom::new(String::new(), None).format_tags();
    match (VolumeEncoding::HuPackedRg8 { offset: 1100.0 }) {
        VolumeEncoding::HuPackedRg8 { offset } => assert_eq!(offset, 1100.0),
        VolumeEncoding::HuFloat => {},
    }
}
fn published_pass_annotation<'pass, 'encoder>(context: PassContext<'pass, 'encoder>) { let _ = context.pass_id; }

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
fn main() {
    release_consumers();
    #[cfg(not(target_arch = "wasm32"))]
    if std::env::args().any(|a| a == "--smoke") {
        use kepler_wgpu::{core::coord::Base, data::ct_volume::CTVolume, get_render_app};
        use std::{thread, time::Duration};
        let hold=std::env::args().any(|a|a=="--hold");
        let mut app=futures::executor::block_on(get_render_app()).expect("synthetic native GPU-ready startup");
        let controls=app.get_glcanvas();
        let values=(0..8).flat_map(|z|(0..16).flat_map(move|y|(0..16).map(move|x|(-1024+128*x+8*y+z) as i16))).collect();
        controls.load_data_from_ct_volume(&CTVolume::new((16,16,8),(1.,1.,1.),values,Base::default()));
        thread::spawn(move || {
            let pause=||thread::sleep(Duration::from_millis(800));
            pause();
            if hold {eprintln!("SMOKE waiting for OS visibility checks");thread::sleep(Duration::from_secs(35));}
            for mode in [0,1,2,3,4] {
                eprintln!("SMOKE mode {mode}");
                controls.set_render_mode(mode,if mode>2{Some(3)}else{None},if mode>2{Some(0)}else{None},if mode>2{Some(2)}else{None},0);
                if mode==1 {for projection in [0,1,2]{controls.set_mip_mode(0,projection);pause();}}
                pause();
            }
            controls.set_window_level(0,400.);
            controls.set_window_width(0,1500.);
            controls.set_scale(0,1.2);
            controls.set_pan(0,0.1,-0.1);
            controls.resize(640,480);
            pause();
            controls.set_render_mode(0,None,None,None,9);
            pause();
            controls.set_render_mode(0,None,None,None,0);
            pause();
            controls.quit();
        });
        futures::executor::block_on(app.run());
        eprintln!("SMOKE normal return");
    }
}
