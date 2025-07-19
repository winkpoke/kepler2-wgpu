#![feature(duration_millis_float)]

use log::{debug, error, info, warn};
use winit::event_loop;
use winit::event_loop::EventLoopBuilder;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::LazyLock;

// use wgpu::util::DeviceExt;
use winit::{
    dpi::PhysicalSize,
    event::*,
    event_loop::EventLoop,
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowBuilder},
};

use view::Renderable;

// mod texture;
pub mod coord;
pub mod ct_volume;
pub mod dicom;
pub mod geometry;
pub mod gl_canvas;
pub mod state;
mod render_content;
mod view;

use ct_volume::CTVolume;
use state::State;
use gl_canvas::RenderApp;


#[cfg(target_arch = "wasm32")]
use async_lock::Mutex;
#[cfg(not(target_arch = "wasm32"))]
use tokio::sync::Mutex;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg_attr(target_arch = "wasm32", wasm_bindgen(start))]
#[cfg(target_arch = "wasm32")]
pub async fn init() {
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));
    console_log::init_with_level(log::Level::Info).expect("Couldn't initialize logger");
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub async fn get_render_app(vol: &CTVolume) -> RenderApp {
    #[cfg(not(target_arch = "wasm32"))]
    env_logger::init();

    warn!("Start the program ...");

    let event_loop = EventLoopBuilder::<gl_canvas::UserEvent>::with_user_event().build().unwrap();
    let window = Arc::new(WindowBuilder::new().build(&event_loop).unwrap());
    let proxy = event_loop.create_proxy();

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

    // Set the window size to 900x900
    let _ = window.request_inner_size(PhysicalSize::new(800, 800));
    let state = State::new(window.clone(), &vol).await;
    RenderApp::new(state, event_loop, proxy)
}