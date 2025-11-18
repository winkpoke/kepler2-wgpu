#![allow(dead_code)]


use std::sync::Arc;

use winit::{
    dpi::PhysicalSize,
    event::*,
    event_loop::EventLoop,
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowBuilder},
};

use crate::{application::App, rendering::LayoutContainer};
use crate::rendering::core::Graphics;
use crate::application::gl_canvas::{GLCanvas, UserEvent};
use winit::event_loop::EventLoopProxy;


#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen_futures::spawn_local;

pub async fn create_graphics(window: Arc<Window>) -> Result<Graphics, crate::core::error::KeplerError> {
    Graphics::new(window).await
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub struct RenderApp {
    pub(crate) state: Option<App>,
    pub(crate) event_loop: Option<EventLoop<UserEvent>>,
    pub(crate) proxy: Option<EventLoopProxy<UserEvent>>,
}

impl RenderApp {
    pub fn new(state: App, event_loop: EventLoop<UserEvent>) -> Self {
        let proxy = event_loop.create_proxy();
        RenderApp {
            state: Some(state),
            event_loop: Some(event_loop),
            proxy: Some(proxy),
        }
    }
    
    pub async fn set_window(&mut self, window: Arc<Window>) {
        if let Some(state) = &mut self.state {
            match Graphics::new(window.clone()).await {
                Ok(graphics) => {
                    state.swap_graphics(graphics);
                    log::info!("Graphics swapped successfully.");
                },
                Err(e) => log::error!("Failed to create graphics: {}", e),
            }
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
        // Take ownership to avoid borrowing `self` inside the event loop closure
        let event_loop = self.event_loop.take().unwrap();
        let mut state = self.state.take().unwrap();
        let proxy = self.proxy.take().unwrap();

        let mut surface_configured = false;

        log::info!("Starting the event loop ...");

        // Request initial redraw to start the rendering loop
        state.window().request_redraw();

        event_loop.run(move |event, target| {
            match event {
                // Event::UserEvent(UserEvent::SetSliceSpeed(index, speed)) => {
                //     state.set_slice_speed(index, speed);
                //     log::info!("Slice speed set to: {}", speed);
                // }
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
                    log::info!("Pan set to: dx={dx}, dy={dy}");
                }
                Event::UserEvent(UserEvent::SetPanMM(index, dx_mm, dy_mm)) => {
                    state.set_pan_mm(index, dx_mm, dy_mm);
                    log::info!("PanMM set to: dx_mm={dx_mm}, dy_mm={dy_mm}");
                }
                Event::UserEvent(UserEvent::Quit) => {
                    log::info!("Quit event received. Exiting event loop.");
                    state.app_view.layout.remove_all();
                    target.exit();
                }
                Event::UserEvent(UserEvent::SetWindowByDivId(div_id, volume)) => {
                    log::info!("SetWindowByDivId event received for div_id: {div_id}");
                    #[cfg(not(target_arch = "wasm32"))]
                    {
                        // Silence unused variable in native builds; volume is used on web.
                        let _ = &volume;
                    }
                    #[cfg(target_arch = "wasm32")]
                    {
                        let window = Arc::new(WindowBuilder::new().build(target).unwrap());
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
                        state.app_view.layout.remove_all();
                        spawn_local(async move {
                            match Graphics::new(window.clone()).await {
                                Ok(graphics) => {
                                    log::info!("Graphics created in SetWindowByDivId event.");
                                    let _ = proxy.send_event(UserEvent::GraphicsReady(graphics, volume));
                                }
                                Err(e) => {
                                    log::error!("Failed to create graphics in SetWindowByDivId: {:?}", e);
                                }
                            }
                        });
                    }
                }
                Event::UserEvent(UserEvent::GraphicsReady(graphics, volume)) => {
                    log::info!("GraphicsReady event received.");
                    state.swap_graphics(graphics);
                    state.resize(PhysicalSize { width: 800, height: 800 });
                    proxy.send_event(UserEvent::LoadDataFromCTVolume(volume)).unwrap();
                    log::info!("Graphics swapped in state.");
                }
                Event::UserEvent(UserEvent::ClearLayout) => {
                    log::info!("ClearLayout event received.");
                    state.app_view.layout.remove_all();
                }
                Event::UserEvent(UserEvent::ReloadShaders) => {
                    // Function-level comment: Shader reload is now handled by individual render contexts that recreate their pipelines as needed.
                    log::info!("ReloadShaders event: render contexts will rebuild pipelines lazily.");
                }
                Event::UserEvent(UserEvent::InvalidatePipelines) => {
                    // Function-level comment: Pipeline invalidation is now handled by individual render contexts.
                    log::info!("InvalidatePipelines event: render contexts will rebuild pipelines as needed.");
                }
                Event::UserEvent(UserEvent::SetEnableMesh(enabled, mip, change_mpr, index_1, index_2, index_3, index_4, downsample, iso_value)) => {
                    // Function-level comment: Runtime mesh toggle via user event; swap slot 2 view accordingly.
                    state.set_mesh_mode_enabled(enabled, mip, change_mpr, index_1, index_2, index_3, index_4, downsample, iso_value);
                    log::info!("EnableMesh toggled at runtime: {}", enabled);
                }
                Event::UserEvent(UserEvent::SetOneCellLayout(mode, orientation_index, downsample, iso_value)) => {
                    // Function-level comment: Runtime mesh toggle via user event; swap slot 2 view accordingly.
                    state.set_one_cell_layout(mode, orientation_index, downsample, iso_value);
                    log::info!("OneCellLayout set to: mode={mode}, orientation_index={orientation_index}, downsample={downsample}, iso_value={iso_value}");
                }
                Event::UserEvent(UserEvent::SetCenterAtPointInMM(index, x_mm, y_mm, z_mm)) => {
                    state.set_center_at_point_in_mm(index, x_mm, y_mm, z_mm);
                    log::info!("CenterAtPointInMM set to: x_mm={x_mm}, y_mm={y_mm}, z_mm={z_mm}");
                }
                // Mesh control events
                Event::UserEvent(UserEvent::SetMeshRotationEnabled(_index, enabled)) => {
                    state.set_mesh_rotation_enabled(enabled);
                    log::info!("Mesh rotation enabled={}", enabled);
                }
                Event::UserEvent(UserEvent::SetMeshPan(_index, dx, dy)) => {
                    state.set_mesh_pan(dx, dy);
                    log::info!("Mesh pan set to dx={:.3}, dy={:.3}", dx, dy);
                }
                Event::UserEvent(UserEvent::ResetMesh(_index)) => {
                    state.reset_mesh();
                    log::info!("Mesh rotation reset");
                }
                Event::UserEvent(UserEvent::SetMeshScale(_index, scale)) => {
                    state.set_mesh_scale(scale);
                    log::info!("Mesh scale set to {:.3}", scale);
                }
                Event::UserEvent(UserEvent::SetMeshRotationAngleDeg(_index, degrees_x, degrees_y, degrees_z)) => {
                    state.set_mesh_rotation_angle_degrees(degrees_x, degrees_y, degrees_z);
                    log::info!("Mesh rotation angle set to {:?}°", [degrees_x, degrees_y, degrees_z]);
                }
                Event::UserEvent(UserEvent::ViewClick(view_index, screen_x, screen_y, screen_z)) => {
                    state.handle_view_click(view_index, screen_x, screen_y, screen_z);
                    log::info!("ViewClick processed for view {}: screen_x={screen_x}, screen_y={screen_y}, screen_z={screen_z}", view_index);
                }
                #[cfg(target_arch = "wasm32")]
                Event::UserEvent(UserEvent::ViewClickGet(view_index, screen_x, screen_y, screen_z, sender)) => {
                    // Function-level comment: Compute view click result and send it back to JS via oneshot channel.
                    let result = state.handle_view_click(view_index, screen_x, screen_y, screen_z);
                    if let Err(_) = sender.send(result) {
                        log::error!("Failed to send ViewClickGet result for view {}", view_index);
                    } else {
                        log::info!("Sent ViewClickGet result for view {}: {:?}", view_index, result);
                    }
                }
                #[cfg(target_arch = "wasm32")]
                Event::UserEvent(UserEvent::GetScreenCoordInMM(index, coord, sender)) => {
                    // Function-level comment: Handle get_screen_coord_in_mm request and send result back via oneshot channel.
                    let result = state.get_screen_coord_in_mm(index, coord);
                    if let Err(_) = sender.send(result) {
                        log::error!("Failed to send GetScreenCoordInMM result for window {}", index);
                    } else {
                        log::info!("Sent GetScreenCoordInMM result for window {}: {:?}", index, result);
                    }
                }
                #[cfg(target_arch = "wasm32")]
                Event::UserEvent(UserEvent::GetWindowLevel(index, sender)) => {
                    let result = state.get_window_level(index);
                    if let Err(_) = sender.send(result) {
                        log::error!("Failed to send GetWindowLevel result for window {}", index);
                    } else {
                        log::info!("Sent GetWindowLevel result for window {}: {:?}", index, result);
                    }
                }
                #[cfg(target_arch = "wasm32")]
                Event::UserEvent(UserEvent::WorldCoordToScreen(index, coord, sender)) => {
                    // Function-level comment: Handle world_coord_to_screen request and send result back via oneshot channel.
                    let result = state.world_coord_to_screen(index, coord);
                    if let Err(_) = sender.send(result) {
                        log::error!("Failed to send WorldCoordToScreen result for window {}", index);
                    } else {
                        log::info!("Sent WorldCoordToScreen result for window {}: {:?}", index, result);
                    }
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
                                // Function-level comment: On 'R' key press, request shader reload via event.
                                if let Err(e) = proxy.send_event(UserEvent::ReloadShaders) {
                                    log::error!("Failed to send ReloadShaders event on KeyR: {:?}", e);
                                } else {
                                    log::info!("KeyR pressed: ReloadShaders event sent");
                                }
                            }
                            WindowEvent::KeyboardInput {
                                event:
                                    KeyEvent {
                                        state: ElementState::Pressed,
                                        physical_key: PhysicalKey::Code(KeyCode::KeyF),
                                        ..
                                    },
                                ..
                            } => {
                                // Disable the runtime toggle feature per requirement
                                state.disable_volume_format_toggle();
                                println!("F key pressed: volume format toggle feature disabled");
                            }
                            WindowEvent::KeyboardInput {
                                event:
                                    KeyEvent {
                                        state: ElementState::Pressed,
                                        physical_key: PhysicalKey::Code(KeyCode::KeyM),
                                        ..
                                    },
                                ..
                            } => {
                                // Function-level comment: Toggle mesh mode on 'M' key press at runtime.
                                let new_enabled = !state.mesh_mode_enabled();
                                state.set_mesh_mode_enabled(new_enabled, None, false, 0, 0, 0, 0, 3, 100.0);
                                log::info!("KeyM pressed: mesh mode toggled to {}", new_enabled);
                            }
                            WindowEvent::RedrawRequested => {
                                // This tells winit that we want another frame after this one
                                state.window().request_redraw();

                                if !surface_configured {
                                    return;
                                }
                                state.update();
                                match state.render() {
                                    Ok(_) => {}
                                    // Reconfigure the surface if it's lost or outdated
                                    Err(
                                        wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated,
                                    ) => {
                                        let width = state.graphics_context.graphics.surface_config.width;
                                        let height = state.graphics_context.graphics.surface_config.height;
                                        let size = PhysicalSize::<u32> {width, height};
                                        // Function-level comment: Surface reconfiguration handled by individual render contexts.
                                        log::info!("Surface error {:?} - render contexts will rebuild pipelines as needed", "Lost/Outdated");
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
