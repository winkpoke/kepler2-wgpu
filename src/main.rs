use kepler_wgpu::{
    data::{ct_volume::CTVolumeGenerator, dicom::fileio},
    get_render_app,
};

#[cfg(not(target_arch = "wasm32"))]
#[tokio::main]
async fn main() {
    env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("info"),
    ).init();

    let port: u16 = std::env::var(kepler_wgpu::server::PORT_ENV_VAR)
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(kepler_wgpu::server::DEFAULT_PORT);

    let server_mode = std::env::args().any(|arg| arg == "server");

    if server_mode {
        log::info!("Starting Kepler2-WGPU in server mode on port {}", port);
        start_server(port).await;
    } else {
        log::info!("Starting Kepler2-WGPU in native desktop mode");
        start_native().await;
    }
}

#[cfg(not(target_arch = "wasm32"))]
async fn start_server(port: u16) {
    use kepler_wgpu::server::{create_router, ServerState};

    let state = ServerState::new();
    let app = create_router(state);

    let addr = format!("0.0.0.0:{port}");
    log::info!("Starting Kepler2-WGPU in server mode");
    log::info!("Axum server listening on http://{addr}");
    log::info!("Access the web interface at http://localhost:{port}/index.html");

    let listener = match tokio::net::TcpListener::bind(&addr).await {
        Ok(l) => l,
        Err(e) => {
            log::error!("Failed to bind to {addr}: {e}");
            std::process::exit(1);
        }
    };

    let make_service = app.into_make_service_with_connect_info::<std::net::SocketAddr>();
    if let Err(e) = axum::serve(listener, make_service).tcp_nodelay(true).await {
        log::error!("Server error: {e}");
        std::process::exit(1);
    }
}

#[cfg(not(target_arch = "wasm32"))]
async fn start_native() {
    let dicom_folders = vec!["C:\\share\\imrt", "C:\\share\\head_mold"];
    let image_series_code = "1.2.392.200036.9116.2.5.1.144.3437232930.1426478676.964561";
    let repo = fileio::parse_dcm_directories(dicom_folders).await.unwrap();
    let vol = repo.generate_ct_volume(image_series_code).unwrap();

    let mut render_app = get_render_app().await.expect("Failed to create render app");
    let gl_canvas = render_app.get_glcanvas();
    gl_canvas.load_data_from_ct_volume(&vol);

    render_app.run().await;
}
