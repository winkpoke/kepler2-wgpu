#[cfg(not(target_arch = "wasm32"))]
extern crate wgpu as wgpu30;

// Allow unit and integration regressions to share the same headless GPU fixture.
#[cfg(test)]
extern crate self as kepler_wgpu;

use log::warn;
use std::sync::Arc;
use winit::event_loop::EventLoop;

// use wgpu::util::DeviceExt;
use winit::{
    dpi::PhysicalSize,
    window::{Window, WindowAttributes},
};

// New module organization
pub mod acquisition;
pub mod application;
pub mod core;
pub mod data;
pub mod rendering;

#[cfg(not(target_arch = "wasm32"))]
pub mod server;

#[cfg(not(target_arch = "wasm32"))]
pub mod gpu;

// All modules now properly organized according to the new architecture

// Re-export commonly used types for backward compatibility
pub use application::{gl_canvas::GLCanvas, render_app::RenderApp, App};
pub use core::{coord, error::KeplerError, timing};
pub use data::volume_encoding::{pack_i16_rg8, unpack_rg8_i16, VolumeEncoding};
pub use data::{ct_volume, dicom};
pub use rendering::view::{Renderable, StaticLayout, View};

// Mesh functionality is now always available
pub use rendering::mesh;

// Current imports for existing functionality
use application::gl_canvas::UserEvent;
use data::ct_volume::CTVolume;

#[cfg(target_arch = "wasm32")]
use async_lock::Mutex;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

use log::LevelFilter;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsValue;

/// Initialize cross-platform logger
/// - Native: env_logger with module filters for wgpu, wgpu_core, wgpu_hal, naga at WARN
/// - WASM: custom logger routing to web_sys::console with the same filters
/// Ensures that adapter/backend and surface format logs from Graphics::initialize are visible at Info level across platforms.
#[cfg(target_arch = "wasm32")]
struct WasmLogger;

#[cfg(target_arch = "wasm32")]
static WASM_LOGGER: WasmLogger = WasmLogger;

#[cfg(any(target_arch = "wasm32", test))]
fn browser_log_enabled(target: &str, level: log::Level) -> bool {
    if target.starts_with("wgpu") || target.starts_with("naga") {
        level <= log::Level::Warn
    } else {
        true
    }
}

#[cfg(target_arch = "wasm32")]
impl log::Log for WasmLogger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        browser_log_enabled(metadata.target(), metadata.level())
    }
    fn log(&self, record: &log::Record) {
        if !self.enabled(record.metadata()) {
            return;
        }
        let msg = format!("{}: {}", record.target(), record.args());
        match record.level() {
            log::Level::Error => web_sys::console::error_1(&JsValue::from_str(&msg)),
            log::Level::Warn => web_sys::console::warn_1(&JsValue::from_str(&msg)),
            log::Level::Info => web_sys::console::log_1(&JsValue::from_str(&msg)),
            log::Level::Debug => web_sys::console::log_1(&JsValue::from_str(&msg)),
            log::Level::Trace => web_sys::console::log_1(&JsValue::from_str(&msg)),
        }
    }
    fn flush(&self) {}
}

#[cfg(not(target_arch = "wasm32"))]
pub fn init_logger() -> Result<(), log::SetLoggerError> {
    let mut builder = env_logger::Builder::new();

    // Check if RUST_LOG is set, if not default to Info level
    if std::env::var("RUST_LOG").is_ok() {
        // If RUST_LOG is set, use env_logger's default behavior which respects it
        builder.parse_default_env();
    } else {
        // If RUST_LOG is not set, use our default configuration
        builder.filter_level(LevelFilter::Info);
    }

    // Always filter noisy wgpu modules to WARN level regardless of RUST_LOG
    builder
        .filter_module("wgpu", LevelFilter::Warn)
        .filter_module("wgpu_core", LevelFilter::Warn)
        .filter_module("wgpu_hal", LevelFilter::Warn)
        .filter_module("naga", LevelFilter::Warn);
    builder.try_init()
}

#[cfg(target_arch = "wasm32")]
pub fn init_logger() -> Result<(), log::SetLoggerError> {
    log::set_logger(&WASM_LOGGER)?;
    log::set_max_level(LevelFilter::Debug);
    Ok(())
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen(start))]
#[cfg(target_arch = "wasm32")]
pub async fn init() {
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));
    if let Err(e) = init_logger() {
        web_sys::console::error_1(&JsValue::from_str(&format!(
            "Logger initialization failed: {}",
            e
        )));
    }
}

/// Compatibility bridge for the existing GPU-ready async constructor.
/// All subsequent window creation uses the active event loop.
#[expect(
    deprecated,
    reason = "preserve GPU-ready construction before run for existing native/browser callers"
)]
fn create_bootstrap_window(
    event_loop: &EventLoop<UserEvent>,
    attributes: WindowAttributes,
) -> Result<Window, KeplerError> {
    event_loop
        .create_window(attributes)
        .map_err(|error| KeplerError::Window(format!("create initial window: {error}")))
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub async fn get_render_app() -> Result<RenderApp, KeplerError> {
    #[cfg(not(target_arch = "wasm32"))]
    if let Err(e) = init_logger() {
        eprintln!("Logger initialization failed: {}", e);
    }

    warn!("Start the program ...");

    let event_loop = EventLoop::<UserEvent>::with_user_event()
        .build()
        .map_err(|error| KeplerError::Window(format!("create initial event loop: {error}")))?;
    let window = Arc::new(create_bootstrap_window(
        &event_loop,
        Window::default_attributes()
            .with_title("Kepler WGPU Medical Imaging")
            .with_inner_size(winit::dpi::LogicalSize::new(800, 800))
            .with_visible(true),
    )?);
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
            .ok_or_else(|| KeplerError::Window("append initial canvas to wasm-example: target, document, or canvas unavailable".into()))?;
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
    let state = App::new(window.clone()).await?;
    Ok(RenderApp::new(state, event_loop))
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub fn drop_render_app(app: RenderApp) {
    drop(app);
}

#[cfg(test)]
mod browser_logger_tests {
    use super::browser_log_enabled;
    use log::Level;

    #[test]
    fn gpu_filter_keeps_errors_and_warnings_and_suppresses_verbose_levels() {
        for target in [
            "wgpu",
            "wgpu_core::instance",
            "wgpu_hal::gles",
            "naga::front",
        ] {
            for level in [
                Level::Error,
                Level::Warn,
                Level::Info,
                Level::Debug,
                Level::Trace,
            ] {
                assert_eq!(
                    browser_log_enabled(target, level),
                    matches!(level, Level::Error | Level::Warn)
                );
            }
        }
        for level in [
            Level::Error,
            Level::Warn,
            Level::Info,
            Level::Debug,
            Level::Trace,
        ] {
            assert!(browser_log_enabled("kepler_wgpu::application", level));
        }
    }
}
