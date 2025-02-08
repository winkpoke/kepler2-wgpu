#![feature(duration_millis_float)]

use log::{debug, error, info, warn};
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

use view::{CoronalView, Layout, ObliqueView, Renderable, SagittalView, TransverseView};

// mod texture;
pub mod coord;
pub mod ct_volume;
pub mod dicom;
pub mod geometry;
pub mod state;
mod render_content;
mod view;

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use ct_volume::*;
use dicom::*;
use state::*;

use std::time::Instant;

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
pub async fn run(vol: &CTVolume) {
    #[cfg(not(target_arch = "wasm32"))]
    env_logger::init();

    warn!("Start the program ...");

    let event_loop = EventLoop::new().unwrap();
    let window = WindowBuilder::new().build(&event_loop).unwrap();

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

    // Set the window size to 1200x900
    let _ = window.request_inner_size(PhysicalSize::new(1200, 900));
    // let mut state = State::initialize(&window).await;
    
    // #[cfg(not(target_arch = "wasm32"))]
    // {
    //     let seconds = 5;
    //     println!("Waiting for {} seconds...", seconds);
    //     // Sleep for `seconds` seconds (non-blocking)
    //     // tokio::time::sleep(std::time::Duration::from_secs(seconds)).await;

    //     // Call the delayed function after the sleep
    //     // state.load_data().await;
    //     state.load_data().await;
    // }

    let mut state = State::new(&window, &vol).await;


    let mut surface_configured = false;

    log::info!("Starting the event loop ...");
    event_loop.run(move |event, control_flow| {
        match event {
            Event::WindowEvent {
                ref event,
                window_id,
            } if window_id == state.window().id() => {
                if !state.input(event) {
                    // UPDATED!
                    match event {
                        WindowEvent::CloseRequested
                        | WindowEvent::KeyboardInput {
                            event:
                                KeyEvent {
                                    state: ElementState::Pressed,
                                    physical_key: PhysicalKey::Code(KeyCode::Escape),
                                    ..
                                },
                            ..
                        } => control_flow.exit(),
                        WindowEvent::Resized(physical_size) => {
                            log::info!("physical_size: {physical_size:?}");
                            surface_configured = true;
                            state.resize(*physical_size);
                        }
                        WindowEvent::KeyboardInput {
                            event:
                                KeyEvent {
                                    state: ElementState::Pressed,
                                    physical_key: PhysicalKey::Code(KeyCode::KeyR),
                                    ..
                                },
                            ..
                        } => {
                            // state = State::initialize(&window).await;
                            println!("R key pressed");
                        }
                        WindowEvent::RedrawRequested => {
                            // This tells winit that we want another frame after this one
                            state.window().request_redraw();

                            if (!surface_configured) {
                                return;
                            }

                            state.update();
                            match state.render() {
                                Ok(_) => {}
                                // Reconfigure the surface if it's lost or outdated
                                Err(
                                    wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated,
                                ) => state.resize(state.size),
                                // The system is out of memory, we should probably quit
                                Err(wgpu::SurfaceError::OutOfMemory) => {
                                    log::error!("OutOfMemory");
                                    control_flow.exit();
                                }

                                // This happens when the a frame takes too long to present
                                Err(wgpu::SurfaceError::Timeout) => {
                                    log::warn!("Surface timeout")
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }).unwrap();
}
