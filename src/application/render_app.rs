#![allow(dead_code)]

use std::sync::Arc;

use winit::{
    dpi::PhysicalSize,
    event::*,
    event_loop::EventLoop,
    keyboard::{KeyCode, PhysicalKey},
    window::Window,
};

#[cfg(target_arch = "wasm32")]
use winit::window::WindowBuilder;

use crate::application::gl_canvas::{GLCanvas, UserEvent};
use crate::rendering::core::Graphics;
use crate::{application::App, rendering::LayoutContainer};
use winit::event_loop::EventLoopProxy;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen_futures::spawn_local;

pub async fn create_graphics(
    window: Arc<Window>,
) -> Result<Graphics, crate::core::error::KeplerError> {
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
                }
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
                    if let Err(e) = state.load_data_from_ct_volume(&volume) {
                        log::error!("Failed to load data from CTVolume: {}", e);
                    } else {
                        log::info!("Loaded data from CTVolume");
                    }
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
                Event::UserEvent(UserEvent::SetRenderMode(mode, mesh_index,mpr_index,mip_index,orientation_index)) => {
                    // Function-level comment: Runtime mesh toggle via user event; swap slot 2 view accordingly.
                    state.set_render_mode(mode, mesh_index,mpr_index,mip_index,orientation_index);
                    log::info!("SetRenderMode toggled at runtime: mode={mode}, mip={:?}, mesh_index={:?}, mpr_index={:?}, orientation_index={orientation_index}", mip_index, mesh_index, mpr_index);
                }
                //mip control events
                Event::UserEvent(UserEvent::SetMipMode(index, mode)) => {
                    state.set_mip_mode(index, mode);
                    log::info!("MipMode set to: index={index}, mode={mode}");
                }
                Event::UserEvent(UserEvent::SetSlabThickness(index, thickness)) => {
                    state.set_slab_thickness(index, thickness);
                    log::info!("SlabThickness set to: index={index}, thickness={thickness}");
                }
                Event::UserEvent(UserEvent::SetMipRotationAngleDeg(index, roll_deg, yaw_deg, pitch_deg)) => {
                    state.set_mip_rotation_angle_degrees(index, roll_deg, yaw_deg, pitch_deg);
                    log::info!(
                        "MipRotationAngleDeg set to: index={index}, roll_deg={roll_deg}, yaw_deg={yaw_deg}, pitch_deg={pitch_deg}"
                    );
                }
                Event::UserEvent(UserEvent::SetObliqueRotation(index, horizontal_radians, vertical_radians, in_plane_radians)) => {
                    state.set_oblique_rotation_radians(index, horizontal_radians, vertical_radians, in_plane_radians);
                    log::info!(
                        "ObliqueRotation set to: index={index}, horizontal={:?}, vertical={:?}, in_plane={:?}",
                        horizontal_radians, vertical_radians, in_plane_radians
                    );
                }
                // Mesh control events
                Event::UserEvent(UserEvent::SetMeshRotationEnabled(_index, enabled)) => {
                    state.set_mesh_rotation_enabled(enabled);
                    log::info!("Mesh rotation enabled={}", enabled);
                }
                Event::UserEvent(UserEvent::ResetMesh(_index)) => {
                    state.reset_mesh();
                    log::info!("Mesh rotation reset");
                }
                Event::UserEvent(UserEvent::SetMeshOpacity(_index, alpha)) => {
                    state.set_mesh_opacity(alpha);
                    log::info!("Mesh opacity set to {:.3}", alpha);
                }
                Event::UserEvent(UserEvent::SetMeshRoi(_index, sx ,sy , sz, lx, ly, lz)) => {
                    state.set_mesh_roi(sx ,sy , sz, lx, ly, lz);
                    log::info!("Mesh roi set from {:?} to {:?}", [sx ,sy , sz], [lx, ly, lz]);
                }
                Event::UserEvent(UserEvent::SetMeshRotationAngleDeg(_index, degrees_x, degrees_y)) => {
                    state.set_mesh_rotation_angle_degrees(degrees_x, degrees_y);
                    log::info!("Mesh rotation angle set to {:?}°", [degrees_x, degrees_y]);
                }
                Event::UserEvent(UserEvent::SetRotationQuat(index, q)) => {
                    state.set_rotation(index, q);
                    log::debug!("Rotation set to {:?}", q);
                }
                Event::UserEvent(UserEvent::SetMeshRotationDegrees(_index, roll_deg, yaw_deg, pitch_deg)) => {
                    state.set_mesh_rotation_degrees(roll_deg, yaw_deg, pitch_deg);
                    log::debug!("Mesh rotation set to {:?}°", [roll_deg, yaw_deg, pitch_deg]);
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
                Event::UserEvent(UserEvent::GetRotation(index, sender)) => {
                    let result = state.get_rotation(index);
                    if let Err(_) = sender.send(result) {
                        log::error!("Failed to send GetRotation result for window {}", index);
                    } else {
                        log::info!("Sent GetRotation result for window {}: {:?}", index, result);
                    }
                }
                #[cfg(target_arch = "wasm32")]
                Event::UserEvent(UserEvent::GetPan(index, sender)) => {
                    let result = state.get_translate_in_screen_coord(index);
                    if let Err(_) = sender.send(result) {
                        log::error!("Failed to send GetPan result for window {}", index);
                    } else {
                        log::info!("Sent GetPan result for window {}: {:?}", index, result);
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
                                        physical_key: PhysicalKey::Code(KeyCode::KeyT),
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
                                        physical_key: PhysicalKey::Code(KeyCode::KeyA),
                                        ..
                                    },
                                ..
                            } => {
                                state.set_render_mode(0, None, None, None, 2);
                                log::info!("KeyA pressed: mpr mode toggled to {}", true);
                            }
                            WindowEvent::KeyboardInput {
                                event:
                                    KeyEvent {
                                        state: ElementState::Pressed,
                                        physical_key: PhysicalKey::Code(KeyCode::KeyB),
                                        ..
                                    },
                                ..
                            } => {
                                state.set_render_mode(2, Some(0), None, None, 1);
                                state.set_mesh_rotation_angle_degrees(90.0, 0.0);
                                // state.set_mesh_rotation_angle_degrees(0.0, 90.0);
                                // state.set_mesh_rotation_angle_degrees(90.0, 0.0);
                                // state.set_mesh_scale(2.0);
                                // state.set_mesh_pan(0.2, 0.2);
                                state.set_mesh_opacity(1.0);
                                state.set_scale(0, 2.0);
                                state.set_window_width(0, 1500.0);
                                state.set_window_level(0, 400.0);
                                log::info!("KeyB pressed: 2*2 mode toggled to {}", true);
                            }
                            WindowEvent::KeyboardInput {
                                event:
                                    KeyEvent {
                                        state: ElementState::Pressed,
                                        physical_key: PhysicalKey::Code(KeyCode::KeyC),
                                        ..
                                    },
                                ..
                            } => {
                                state.set_render_mode(2, Some(0), None, None, 1);
                                state.set_mesh_rotation_degrees(-90.0, 90.0, 0.0);
                                // state.set_mesh_roi(0.0,0.0,0.5,1.0,1.0,1.0);
                                state.set_mesh_opacity(1.0);
                                state.set_scale(0, 2.0);
                                state.set_window_width(0, 300.0);
                                state.set_window_level(0, 300.0);
                                log::info!("KeyC pressed: mesh mode toggled to {}", true);
                            }
                            WindowEvent::KeyboardInput {
                                event:
                                    KeyEvent {
                                        state: ElementState::Pressed,
                                        physical_key: PhysicalKey::Code(KeyCode::KeyD),
                                        ..
                                    },
                                ..
                            } => {
                                state.set_render_mode(3, Some(0), None, Some(3), 1);
                                state.set_mip_mode(3,0);
                                state.set_slab_thickness(3, 1.25);
                                state.set_scale(3, 2.0);
                                state.set_pan(3, 0.2, 0.2);
                                state.set_mip_rotation_angle_degrees(3, 0.0, 180.0, 90.0);
                                log::info!("KeyD pressed: 2*2 mode toggled to {}", true);
                            }
                            WindowEvent::KeyboardInput {
                                event:
                                    KeyEvent {
                                        state: ElementState::Pressed,
                                        physical_key: PhysicalKey::Code(KeyCode::KeyE),
                                        ..
                                    },
                                ..
                            } => {
                                state.set_render_mode(1, None, None, Some(0), 1);
                                log::info!("KeyE pressed: mip mode toggled to {}", true);
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
                                state.set_render_mode(3, None, Some(0), None, 1);
                                state.set_slice_mm(0, 100.0);
                                state.set_scale(0, 2.0);
                                state.set_pan(1, 0.09, 0.09);
                                log::info!("KeyF pressed: 2*2 mode toggled to {}", true);
                            }
                            WindowEvent::KeyboardInput {
                                event:
                                    KeyEvent {
                                        state: ElementState::Pressed,
                                        physical_key: PhysicalKey::Code(KeyCode::KeyG),
                                        ..
                                    },
                                ..
                            } => {
                                state.set_render_mode(0, None, None, None, 1);
                                state.set_scale(0, 0.5);
                                log::info!("KeyG pressed: 2*2 mode toggled to {}", true);
                            }
                            WindowEvent::KeyboardInput {
                                event:
                                    KeyEvent {
                                        state: ElementState::Pressed,
                                        physical_key: PhysicalKey::Code(KeyCode::KeyO),
                                        ..
                                    },
                                ..
                            } => {
                                state.set_render_mode(3, None, Some(3), None, 3);
                                // state.set_oblique_normal(3, [-0.574, 0.0, 0.819], 0f32);
                            }
                            WindowEvent::KeyboardInput {
                                event:
                                    KeyEvent {
                                        state: ElementState::Pressed,
                                        physical_key: PhysicalKey::Code(KeyCode::KeyP),
                                        ..
                                    },
                                ..
                            } => {
                                state.set_render_mode(3, None, Some(3), None, 3);
                                state.set_oblique_rotation_radians(3, 0.0, 20.0, 0.0);
                            }
                            WindowEvent::KeyboardInput {
                                event:
                                    KeyEvent {
                                        state: ElementState::Pressed,
                                        physical_key: PhysicalKey::Code(KeyCode::KeyS),
                                        ..
                                    },
                                ..
                            } => {
                                state.set_scale(0, 1.0);
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
                                        let width = state.graphics().surface_config.width;
                                        let height = state.graphics().surface_config.height;
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
