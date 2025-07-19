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
pub struct RenderApp {
    pub(crate) state: State,
    pub(crate) event_loop: Option<EventLoop<UserEvent>>,
    pub(crate) proxy: Option<EventLoopProxy<UserEvent>>,
}

impl RenderApp {
    pub fn new(state: State, event_loop: EventLoop<UserEvent>, proxy: EventLoopProxy<UserEvent>) -> Self {
        RenderApp {
            state,
            event_loop: Some(event_loop),
            proxy: Some(proxy),
        }
    }
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
impl RenderApp {
    pub fn get_glcanvas(&self) -> GLCanvas {
        GLCanvas {
            proxy: self.proxy.clone().unwrap(),
        }
    }

    pub async fn run(&mut self) {
        // Take the event_loop out before borrowing self.state
        let event_loop = self.event_loop.take().unwrap();
        let mut state = &mut self.state;

        let mut surface_configured = false;

        log::info!("Starting the event loop ...");
        event_loop.run(move |event, control_flow| {
            match event {
                Event::UserEvent(UserEvent::SetSliceSpeed(speed)) => {
                    state.set_slice_speed(speed);
                    log::warn!("Slice speed set to: {}", speed);
                }
                Event::UserEvent(UserEvent::SetWindowLevel(window_level)) => {
                    state.set_window_level(window_level);
                    log::warn!("Window level set to: {}", window_level);
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
    SetWindowLevel(f32),
    // ... add more events as needed
}

#[macro_export]
macro_rules! impl_user_event_senders_for_glcanvas {
    ($( $fn_name:ident => $variant:ident($arg:ident : $arg_ty:ty) ),* $(,)?) => {
        #[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
        impl GLCanvas {
            $(
                pub fn $fn_name(&self, $arg: $arg_ty) {
                    if let Err(e) = self.proxy.send_event(UserEvent::$variant($arg)) {
                        log::error!("Failed to send {} event: {:?}", stringify!($variant), e);
                    } else {
                        log::info!("Sent {} event: {:?}", stringify!($variant), $arg);
                    }
                }
            )*
        }
    };
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub struct GLCanvas {
    pub(crate) proxy: EventLoopProxy<UserEvent>,
}

impl_user_event_senders_for_glcanvas! {
    set_slice_speed => SetSliceSpeed(speed: f32),
    set_window_level => SetWindowLevel(window_level: f32),
}