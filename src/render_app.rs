use winit::{
    dpi::PhysicalSize,
    event::*,
    event_loop::EventLoop,
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowBuilder},
};

use crate::state::State;
use crate::gl_canvas::{GLCanvas, UserEvent};
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
                Event::UserEvent(UserEvent::SetSliceSpeed(index, speed)) => {
                    state.set_slice_speed(index, speed);
                    log::info!("Slice speed set to: {}", speed);
                }
                Event::UserEvent(UserEvent::SetWindowLevel(index, window_level)) => {
                    state.set_window_level(index, window_level);
                    log::info!("Window level set to: {}", window_level);
                }
                Event::UserEvent(UserEvent::SetWindowWidth(index, window_width)) => {
                    state.set_window_width(index, window_width);
                    log::info!("Window width set to: {}", window_width);
                }
                Event::UserEvent(UserEvent::SetSliceMM(index, z)) => {
                    state.set_slice_mm(index, z);
                    log::info!("Slice set to: {} mm", z);
                }
                Event::UserEvent(UserEvent::SetScale(index, scale)) => {
                    state.set_scale(index, scale);
                    log::info!("Scale set to: {}", scale);
                }
                Event::UserEvent(UserEvent::SetTranslate(index, dx, dy, dz)) => {
                    let translate = [dx, dy, dz];
                    log::info!("Translate set to: {:#?}", translate);
                    state.set_translate(index, translate);
                }
                Event::UserEvent(UserEvent::SetTranslateInScreenCoord(index, dx, dy, dz)) => {
                    let translate = [dx, dy, dz];
                    log::info!("Move to: {:#?}", translate);
                    state.set_translate_in_screen_coord(index, translate);
                }
                Event::UserEvent(UserEvent::LoadDataFromCTVolume(index, volume)) => {
                    state.load_data_from_ct_volume(&volume);
                    log::info!("Loaded data from CTVolume for window {}", index);
                }
                Event::UserEvent(UserEvent::Resize(width, height)) => {
                    log::info!("Resizing to width: {}, height: {}", width, height);
                    state.resize(PhysicalSize { width, height });
                    surface_configured = true;
                }
                Event::UserEvent(UserEvent::SetPan(index, dx, dy)) => {
                    state.set_pan(index, dx, dy);
                    log::info!("Pan set to: dx={}, dy={}", dx, dy);
                }
                Event::UserEvent(UserEvent::SetPanMM(index, dx_mm, dy_mm)) => {
                    state.set_pan_mm(index, dx_mm, dy_mm);
                    log::info!("PanMM set to: dx_mm={}, dy_mm={}", dx_mm, dy_mm);
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