#![feature(duration_millis_float)]

use log::{debug, error, info, warn};
use winit::event_loop::EventLoopBuilder;
use std::sync::Arc;

// use wgpu::util::DeviceExt;
use winit::{
    dpi::PhysicalSize,
    window::{Window, WindowBuilder},
};

use crate::state::Graphics;
use crate::error::KeplerError;

// mod texture;
pub mod coord;
pub mod ct_volume;
pub mod dicom;
pub mod error;
pub mod geometry;
pub mod gl_canvas;
pub mod state;
pub mod pipeline;
pub mod render_content;
pub mod view;
pub mod render_app;
pub mod pipeline_builder;

// Mesh module is opt-in via Cargo feature `mesh`
#[cfg(feature = "mesh")]
pub mod mesh;

use ct_volume::CTVolume;
use state::State;
use render_app::RenderApp;
use gl_canvas::UserEvent;


#[cfg(target_arch = "wasm32")]
use async_lock::Mutex;
#[cfg(not(target_arch = "wasm32"))]
use tokio::sync::Mutex;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsValue;
use log::LevelFilter;

/// Initialize cross-platform logger
/// - Native: env_logger with module filters for wgpu, wgpu_core, wgpu_hal, naga at WARN
/// - WASM: custom logger routing to web_sys::console with the same filters
/// Ensures that adapter/backend and surface format logs from Graphics::initialize are visible at Info level across platforms.
#[cfg(target_arch = "wasm32")]
struct WasmLogger;

#[cfg(target_arch = "wasm32")]
static WASM_LOGGER: WasmLogger = WasmLogger;

#[cfg(target_arch = "wasm32")]
impl log::Log for WasmLogger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        let t = metadata.target();
        let is_wgpu = t.starts_with("wgpu") || t.starts_with("naga");
        if is_wgpu { metadata.level() >= log::Level::Warn } else { metadata.level() <= log::Level::Info }
    }
    fn log(&self, record: &log::Record) {
        if !self.enabled(record.metadata()) { return; }
        let msg = format!("{}: {}", record.target(), record.args());
        match record.level() {
            log::Level::Error => web_sys::console::error_1(&JsValue::from_str(&msg)),
            log::Level::Warn => web_sys::console::warn_1(&JsValue::from_str(&msg)),
            log::Level::Info => web_sys::console::log_1(&JsValue::from_str(&msg)),
            log::Level::Debug => web_sys::console::debug_1(&JsValue::from_str(&msg)),
            log::Level::Trace => web_sys::console::log_1(&JsValue::from_str(&msg)),
        }
    }
    fn flush(&self) {}
}

#[cfg(not(target_arch = "wasm32"))]
pub fn init_logger() -> Result<(), log::SetLoggerError> {
    let mut builder = env_logger::Builder::new();
    builder
        .filter_level(LevelFilter::Info)
        .filter_module("wgpu", LevelFilter::Warn)
        .filter_module("wgpu_core", LevelFilter::Warn)
        .filter_module("wgpu_hal", LevelFilter::Warn)
        .filter_module("naga", LevelFilter::Warn);
    builder.try_init()
}

#[cfg(target_arch = "wasm32")]
pub fn init_logger() -> Result<(), log::SetLoggerError> {
    log::set_logger(&WASM_LOGGER)?;
    log::set_max_level(LevelFilter::Info);
    Ok(())
}



#[cfg_attr(target_arch = "wasm32", wasm_bindgen(start))]
#[cfg(target_arch = "wasm32")]
pub async fn init() {
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));
    if let Err(e) = init_logger() {
        web_sys::console::error_1(&JsValue::from_str(&format!("Logger initialization failed: {}", e)));
    }
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub async fn get_render_app() -> Result<RenderApp, KeplerError> {
    #[cfg(not(target_arch = "wasm32"))]
    if let Err(e) = init_logger() {
        eprintln!("Logger initialization failed: {}", e);
    }



    warn!("Start the program ...");

    let event_loop = EventLoopBuilder::<UserEvent>::with_user_event().build().unwrap();
    let window = Arc::new(WindowBuilder::new().build(&event_loop).unwrap());
    // let proxy = event_loop.create_proxy();

    #[cfg(target_arch = "wasm32")]
    {
        // Winit prevents sizing with CSS, so we have to set
        // the size manually when on web.
        use winit::dpi::PhysicalSize;
        use winit::platform::web::WindowExtWebSys;
        web_sys::window()
            .and_then(|win| win.document())
            .and_then(|doc| {
                let dst = doc.get_element_by_id("wasm-example")?;
                let canvas = web_sys::Element::from(window.canvas()?);
                dst.append_child(&canvas).ok()?;
                Some(())
            })
            .expect("Couldn't append canvas to document body.");
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        // Set the window size to 800x800
        // the request_inner_size function sets the style width and height of the window canvas
        // in web, the size then is controlled by CSS, which blocks the resize on the web platform.
        // let _ = window.request_inner_size(PhysicalSize::new(800, 800));
    }

    // this sets the style width and height of the canvas
    let _ = window.request_inner_size(PhysicalSize::new(800, 800)); 
    let state = State::new(window.clone()).await?;
    Ok(RenderApp::new(state, event_loop))
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub fn drop_render_app(app: RenderApp) {
    drop(app);
}
