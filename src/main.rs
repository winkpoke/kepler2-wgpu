use kepler_wgpu::{get_render_app, data::{ct_volume::CTVolumeGenerator, dicom::fileio}};
// use kepler_wgpu::gl_canvas::GLCanvas;


#[cfg(not(target_arch="wasm32"))]
#[tokio::main]
async fn main() {  
    let dicom_folders = vec![
        "C:\\share\\imrt",
        "C:\\share\\head_mold",
    ];
    let image_series_code = "1.2.392.200036.9116.2.5.1.144.3437232930.1426478676.964561";
    let repo = fileio::parse_dcm_directories(dicom_folders).await.unwrap();
    let vol = repo.generate_ct_volume(image_series_code).unwrap();

    // pollster::block_on(run());
    let mut render_app = get_render_app().await.expect("Failed to create render app");
    let gl_canvas = render_app.get_glcanvas();
    gl_canvas.load_data_from_ct_volume(&vol);
    gl_canvas.enable_mesh(true, None, false, 0, 1, 2, 0, 3, 100.0);

    // Inject test events for verification
    gl_canvas.set_window_level(0, 40.0);
    gl_canvas.set_window_width(0, 350.0);
    gl_canvas.set_window_level(1, 40.0);
    gl_canvas.set_window_width(1, 350.0);
    gl_canvas.set_window_level(2, 40.0);
    gl_canvas.set_window_width(2, 350.0);
    // gl_canvase.set_slice_mm(0, 5.0);
    // gl_canvase.set_scale(0, 1.25);
    // gl_canvase.set_translate(0, 0.0, 0.0, 0.0);
    // gl_canvase.set_pan(0, 10.0, -5.0);

    render_app.run().await;
}
