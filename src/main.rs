use kepler_wgpu::{get_glcanvas, ct_volume::CTVolumeGenerator, dicom::fileio};
// use kepler_wgpu::gl_canvas::GLCanvas;


#[cfg(not(target_arch="wasm32"))]
#[tokio::main]
async fn main() {
    use kepler_wgpu::get_glcanvas;
   
    let dicom_folders = vec![
        "C:\\share\\imrt",
        "C:\\share\\head_mold",
    ];
    let image_series_code = "1.2.392.200036.9116.2.5.1.144.3437232930.1426478676.964561";
    let repo = fileio::parse_dcm_directories(dicom_folders).await.unwrap();
    let vol = repo.generate_ct_volume(image_series_code).unwrap();

    // pollster::block_on(run());
    let mut gl_canvas = get_glcanvas(&vol).await;
    gl_canvas.run().await;
}
