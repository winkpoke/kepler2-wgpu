use winit::{
    dpi::PhysicalSize,
    event::*,
    event_loop::EventLoop,
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowBuilder},
};

use crate::state::State;
use winit::event_loop::EventLoopProxy;


#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub struct GLCanvas {
    pub(crate) state: State,
    pub(crate) event_loop: Option<EventLoop<UserEvent>>,
    pub(crate) proxy: Option<EventLoopProxy<UserEvent>>,
    pub slice_speed: f32,
}

impl GLCanvas {
    pub fn new(state: State, event_loop: EventLoop<UserEvent>, proxy: EventLoopProxy<UserEvent>) -> Self {
        GLCanvas {
            state,
            event_loop: Some(event_loop),
            proxy: Some(proxy),
            slice_speed: 0.0005,
        }
    }
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
impl GLCanvas {
    pub fn set_slice_speed(&mut self, speed: f32) {
        self.slice_speed = speed;
        self.state.set_slice_speed(speed);
    }

    pub fn send_slice_speed(&self, speed: f32) {
        if let Some(proxy) = &self.proxy {
            match proxy.send_event(UserEvent::SetSliceSpeed(speed)) {
                Ok(()) => log::info!("Sent slice speed event: {}", speed),
                Err(e) => log::error!("Failed to send slice speed event: {:?}", e),
            }
        } else {
            log::warn!("No proxy available to send slice speed event");
        }
    }

    pub async fn run(&mut self) {
        // Take the event_loop out before borrowing self.state
        let event_loop = self.event_loop.take().unwrap();
        let mut state = &mut self.state;

        let mut surface_configured = false;
        let slice_speed = self.slice_speed;

        log::info!("Starting the event loop ...");
        event_loop.run(move |event, control_flow| {
            match event {
                Event::UserEvent(UserEvent::SetSliceSpeed(speed)) => {
                    state.set_slice_speed(speed);
                    log::warn!("Slice speed set to: {}", speed);
                }
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
                                state.set_slice_speed(slice_speed);
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
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub struct AppState {
    pub slice_speed: f32,
}

#[derive(Debug)]
pub enum UserEvent {
    SetSliceSpeed(f32),
    // ... add more events as needed
}