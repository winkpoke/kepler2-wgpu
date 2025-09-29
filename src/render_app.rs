use std::{cell::RefCell, rc::Rc, sync::Arc};

use winit::{
    dpi::PhysicalSize,
    event::*,
    event_loop::EventLoop,
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowBuilder},
};

use crate::{ct_volume, state::{Graphics, State}};
use crate::gl_canvas::{GLCanvas, UserEvent};
use winit::event_loop::EventLoopProxy;


#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen_futures::spawn_local;

pub async fn create_graphics(window: Arc<Window>) -> Graphics {
    Graphics::new(window).await
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub struct RenderApp {
    pub(crate) state: Option<State>,
    pub(crate) event_loop: Option<EventLoop<UserEvent>>,
    pub(crate) proxy: Option<EventLoopProxy<UserEvent>>,
}

impl RenderApp {
    pub fn new(state: State, event_loop: EventLoop<UserEvent>) -> Self {
        let proxy = event_loop.create_proxy();
        RenderApp {
            state: Some(state),
            event_loop: Some(event_loop),
            proxy: Some(proxy),
        }
    }
    
    pub async fn set_window(&mut self, window: Arc<Window>) {
        if let Some(state) = &mut self.state {
            let graphics = Graphics::new(window.clone()).await;
            state.swap_graphics(graphics);
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
        let state = self.state.as_mut().unwrap();
        let proxy = self.proxy.as_mut().unwrap().clone();

        let mut surface_configured = false;

        log::info!("Starting the event loop ...");

        event_loop.run(move |event, target| {
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
                Event::UserEvent(UserEvent::LoadDataFromCTVolume(volume)) => {
                    state.load_data_from_ct_volume(&volume);
                    log::info!("Loaded data from CTVolume");
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
                Event::UserEvent(UserEvent::Quit) => {
                    log::info!("Quit event received. Exiting event loop.");
                    state.layout.remove_all();
                    target.exit();
                }
                Event::UserEvent(UserEvent::SetWindowByDivId(div_id, volume)) => {
                    log::info!("SetWindowByDivId event received for div_id: {}", div_id);

                    let window = Arc::new(WindowBuilder::new().build(target).unwrap());
                    #[cfg(target_arch = "wasm32")]
                    {
                        // Winit prevents sizing with CSS, so we have to set
                        // the size manually when on web.
                        use winit::dpi::PhysicalSize;
                        use winit::platform::web::WindowExtWebSys;
                        web_sys::window()
                            .and_then(|win| win.document())
                            .and_then(|doc| {
                                let dst = doc.get_element_by_id(div_id.as_str())?;
                                let canvas = web_sys::Element::from(window.canvas()?);
                                dst.append_child(&canvas).ok()?;
                                Some(())
                            })
                            .expect("Couldn't append canvas to document body.");
                        let _ = window.request_inner_size(PhysicalSize::new(800, 800)); 
                        let proxy = proxy.clone();
                        state.layout.remove_all();
                        spawn_local(async move {
                            let graphics = Graphics::new(window.clone()).await;
                            log::info!("Graphics created in SetWindowByDivId event. {:?}", graphics);
                            let _ = proxy.send_event(UserEvent::GraphicsReady(graphics, volume));
                        });
                    }
                }
                Event::UserEvent(UserEvent::GraphicsReady(graphics, volume)) => {
                    log::info!("GraphicsReady event received.");
                    state.swap_graphics(graphics);
                    state.resize(PhysicalSize { width: 800, height: 800 });
                    let _ = proxy.send_event(UserEvent::LoadDataFromCTVolume(volume)).unwrap();
                    log::info!("Graphics swapped in state.");
                }
                Event::UserEvent(UserEvent::ClearLayout) => {
                    log::info!("ClearLayout event received.");
                    state.layout.remove_all();
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
                            } => target.exit(),
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
                                    ) => {
                                        let width = state.graphics.surface_config.width;
                                        let height = state.graphics.surface_config.height;
                                        let size = PhysicalSize::<u32> {width, height};
                                        state.resize(size);
                                    }
                                    // The system is out of memory, we should probably quit
                                    Err(wgpu::SurfaceError::OutOfMemory) => {
                                        log::error!("OutOfMemory");
                                        target.exit();
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