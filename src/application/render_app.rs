#![allow(dead_code)]

use std::sync::Arc;

use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::*,
    event_loop::{ActiveEventLoop, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::Window,
};

use crate::application::gl_canvas::{GLCanvas, UserEvent};
use crate::application::viewer_lifecycle::{
    LifecycleEvent, LifecycleHost, PendingWork, ViewerLifecycle, WindowSignal,
};
use crate::core::timing::Instant;
use crate::rendering::core::Graphics;
use crate::{application::App, rendering::LayoutContainer};
use winit::event_loop::ControlFlow;
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
        attach_fault_transport(&state, &proxy);
        RenderApp {
            state: Some(state),
            event_loop: Some(event_loop),
            proxy: Some(proxy),
        }
    }

    pub async fn set_window(&mut self, window: Arc<Window>) {
        if let Some(state) = &mut self.state {
            match Graphics::new(window.clone()).await {
                Ok(graphics) => match state.try_swap_graphics(graphics) {
                    Ok(()) => {
                        if let Some(proxy) = &self.proxy {
                            attach_fault_transport(state, proxy);
                        }
                        log::info!("Graphics swapped successfully.");
                    }
                    Err(e) => log::error!("Failed to swap graphics: {}", e),
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
        let event_loop = self.event_loop.take().unwrap();
        let handler = ViewerHandler {
            state: self.state.take().unwrap(),
            proxy: self.proxy.take().unwrap(),
            lifecycle: ViewerLifecycle::default(),
            clock: Instant::now(),
        };
        log::info!("Starting the event loop ...");
        #[cfg(not(target_arch = "wasm32"))]
        {
            let mut handler = handler;
            if let Err(error) = event_loop.run_app(&mut handler) {
                log::error!("Viewer event loop failed: {error}");
            }
        }
        #[cfg(target_arch = "wasm32")]
        {
            use winit::platform::web::EventLoopExtWebSys;
            event_loop.spawn_app(handler);
        }
    }
}

fn attach_fault_transport(state: &App, proxy: &EventLoopProxy<UserEvent>) {
    let proxy = proxy.clone();
    state.graphics().faults.set_wake(move || {
        let _ = proxy.send_event(UserEvent::Quit);
    });
}

/// One owner for initialized graphics, controls, lifecycle, and frame scheduling.
struct ViewerHandler {
    state: App,
    proxy: EventLoopProxy<UserEvent>,
    lifecycle: ViewerLifecycle,
    clock: Instant,
}

impl ApplicationHandler<UserEvent> for ViewerHandler {
    fn user_event(&mut self, target: &ActiveEventLoop, event: UserEvent) {
        self.dispatch(Event::UserEvent(event), target);
    }
    fn window_event(
        &mut self,
        target: &ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        self.dispatch(Event::WindowEvent { window_id, event }, target);
    }
    fn resumed(&mut self, target: &ActiveEventLoop) {
        self.dispatch(Event::Resumed, target);
    }
    fn suspended(&mut self, target: &ActiveEventLoop) {
        self.dispatch(Event::Suspended, target);
    }
    fn about_to_wait(&mut self, target: &ActiveEventLoop) {
        self.dispatch(Event::AboutToWait, target);
    }
}

impl ViewerHandler {
    fn dispatch(&mut self, event: Event<UserEvent>, target: &ActiveEventLoop) {
        if self.lifecycle.shutting_down() {
            return;
        }
        let state = &mut self.state;
        let proxy = &self.proxy;
        let lifecycle = &mut self.lifecycle;
        let clock = &self.clock;
        let mut lifecycle_event = None;
        match event {
            Event::UserEvent(UserEvent::SetWindowLevel(index, window_level)) => {
                state.set_window_level(index, window_level);
            }
            Event::UserEvent(UserEvent::SetWindowWidth(index, window_width)) => {
                state.set_window_width(index, window_width);
            }
            Event::UserEvent(UserEvent::SetSliceMM(index, z)) => {
                state.set_slice_mm(index, z);
            }
            Event::UserEvent(UserEvent::SetScale(index, scale)) => {
                state.set_scale(index, scale);
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
            Event::UserEvent(UserEvent::LoadDrToPro(index, vol, avg_dark, avg_bright)) => {
                state.load_dr_to_pro(index, &vol, &avg_dark, &avg_bright);
            }
            Event::UserEvent(UserEvent::LoadDrOverlay(overlay_vol_0, overlay_vol_90, scale)) => {
                state.overlay_dr_segmentation(&overlay_vol_0, &overlay_vol_90, scale);
            }
            Event::UserEvent(UserEvent::Resize(width, height)) => {
                log::info!("Resizing to width: {}, height: {}", width, height);
                state.resize(PhysicalSize { width, height });
            }
            Event::UserEvent(UserEvent::SetPan(index, dx, dy)) => {
                state.set_pan(index, dx, dy);
            }
            Event::UserEvent(UserEvent::SetPanMM(index, dx_mm, dy_mm)) => {
                state.set_pan_mm(index, dx_mm, dy_mm);
            }
            Event::UserEvent(UserEvent::SetAliasing(index, aliasing)) => {
                state.set_aliasing(index, aliasing);
            }
            Event::UserEvent(UserEvent::Quit) => {
                log::info!("Quit event received. Exiting event loop.");
                lifecycle_event = Some(LifecycleEvent::Quit);
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
                    let window = match target.create_window(Window::default_attributes()) {
                        Ok(window) => Arc::new(window),
                        Err(error) => {
                            log::error!("Create replacement canvas window: {error}");
                            return;
                        }
                    };
                    // Winit prevents sizing with CSS, so we have to set
                    // the size manually when on web.
                    use winit::dpi::PhysicalSize;
                    use winit::platform::web::WindowExtWebSys;
                    if web_sys::window()
                        .and_then(|win| win.document())
                        .and_then(|doc| {
                            let dst = doc.get_element_by_id(div_id.as_str())?;
                            let canvas = web_sys::Element::from(window.canvas()?);
                            dst.append_child(&canvas).ok()?;
                            Some(())
                        })
                        .is_none()
                    {
                        log::error!("Append replacement canvas: div or canvas unavailable");
                        return;
                    }
                    let _ = window.request_inner_size(PhysicalSize::new(800, 800));
                    let proxy = proxy.clone();
                    spawn_local(async move {
                        match Graphics::new(window.clone()).await {
                            Ok(graphics) => {
                                log::info!("Graphics created in SetWindowByDivId event.");
                                let _ =
                                    proxy.send_event(UserEvent::GraphicsReady(graphics, volume));
                            }
                            Err(e) => {
                                log::error!(
                                    "Failed to create graphics in SetWindowByDivId: {:?}",
                                    e
                                );
                            }
                        }
                    });
                }
            }
            Event::UserEvent(UserEvent::GraphicsReady(graphics, volume)) => {
                log::info!("GraphicsReady event received.");
                lifecycle_event = Some(LifecycleEvent::GraphicsReady(graphics, volume));
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
                log::info!(
                    "InvalidatePipelines event: render contexts will rebuild pipelines as needed."
                );
            }
            Event::UserEvent(UserEvent::SetRenderMode(
                mode,
                mesh_index,
                mpr_index,
                mip_index,
                orientation_index,
            )) => {
                // Function-level comment: Runtime mesh toggle via user event; swap slot 2 view accordingly.
                state.set_render_mode(mode, mesh_index, mpr_index, mip_index, orientation_index);
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
            Event::UserEvent(UserEvent::SetRotationAngleDeg(
                index,
                roll_deg,
                yaw_deg,
                pitch_deg,
            )) => {
                state.set_rotation_angle_degrees(index, roll_deg, yaw_deg, pitch_deg);
                log::info!(
                    "MipRotationAngleDeg set to: index={index}, roll_deg={roll_deg}, yaw_deg={yaw_deg}, pitch_deg={pitch_deg}"
                );
            }
            Event::UserEvent(UserEvent::SetObliqueRotation(
                index,
                horizontal_radians,
                vertical_radians,
                in_plane_radians,
            )) => {
                state.set_oblique_rotation_radians(
                    index,
                    horizontal_radians,
                    vertical_radians,
                    in_plane_radians,
                );
                log::info!("Oblique Rotation radians set to horizontal={horizontal_radians}, vertical={vertical_radians}, in plane={in_plane_radians}");
            }
            Event::UserEvent(UserEvent::SetObliquePlane(index, oblique_crop, alpha)) => {
                state.sync_oblique_to_3d(index, oblique_crop, alpha);
                log::info!(
                    "Oblique view {} plane {} alpha={:?}",
                    index,
                    oblique_crop,
                    alpha
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
            Event::UserEvent(UserEvent::SetMeshRoi(_index, sx, sy, sz, lx, ly, lz)) => {
                state.set_mesh_roi(sx, sy, sz, lx, ly, lz);
                log::info!("Mesh roi set from {:?} to {:?}", [sx, sy, sz], [lx, ly, lz]);
            }
            Event::UserEvent(UserEvent::SetMeshMode(_index, mode)) => {
                state.set_mesh_mode(mode);
                log::info!("Mesh mode set to {:?}", mode);
            }
            Event::UserEvent(UserEvent::SetRotationDeg(index, degrees_x, degrees_y)) => {
                state.set_rotation_degrees(index, degrees_x, degrees_y);
                log::info!(
                    "View {} set rotation_degrees: dx={}, dy={}",
                    index,
                    degrees_x,
                    degrees_y
                );
            }
            Event::UserEvent(UserEvent::SetRotationQuat(index, q)) => {
                state.set_rotation(index, q);
                log::debug!("Rotation set to {:?}", q);
            }
            Event::UserEvent(UserEvent::ViewClick(view_index, screen_x, screen_y, screen_z)) => {
                state.handle_view_click(view_index, screen_x, screen_y, screen_z);
                log::info!("ViewClick processed for view {}: screen_x={screen_x}, screen_y={screen_y}, screen_z={screen_z}", view_index);
            }
            Event::UserEvent(UserEvent::SetMeshBallEnabled(_index, enabled)) => {
                state.set_ball_enabled(enabled);
            }
            Event::UserEvent(UserEvent::SetBall(_index, ball, x, y, z)) => {
                state.set_ball(ball, x, y, z);
            }
            Event::UserEvent(UserEvent::SetMeshNeedleEnabled(_index, enabled)) => {
                state.set_mesh_needle_enabled(enabled);
                log::info!("Mesh needle enabled={}", enabled);
            }
            Event::UserEvent(UserEvent::SetMeshNeedleTrajectory(
                _index,
                id,
                x,
                y,
                z,
                lx,
                ly,
                lz,
                r,
                g,
                b,
            )) => {
                state.set_new_needle_mm(id, x, y, z, lx, ly, lz, r, g, b);
                log::info!("Mesh needle set to {:?}", [x, y, z, lx, ly, lz]);
            }
            Event::UserEvent(UserEvent::SetMeshNeedlePosition(_index, id, x, y, z)) => {
                state.set_needle_position_mm(id, x, y, z);
                log::info!("Mesh needle position set to {:?}", [x, y, z]);
            }
            Event::UserEvent(UserEvent::SetMeshNeedleRadius(_index, id, radius)) => {
                state.set_needle_radius(id, radius);
                log::info!("Mesh needle radius set to {:.3}", radius);
            }
            Event::UserEvent(UserEvent::SetMeshNeedleAngle(index, id, angle)) => {
                state.set_needle_angle(index, id, angle);
                log::info!("Mesh view {} set needle {} angle: {}°", index, id, angle);
            }
            Event::UserEvent(UserEvent::SetSegmentationAll(raw)) => {
                state.set_ai_segmentation(raw);
            }
            Event::UserEvent(UserEvent::SetSegmentationVisibility(m)) => {
                state.set_ai_segmentation_visibility(m);
            }
            Event::UserEvent(UserEvent::SetOBJMesh(raw, id, kind)) => {
                state.set_obj_mesh(raw, id, kind);
            }
            #[cfg(target_arch = "wasm32")]
            Event::UserEvent(UserEvent::ExportCurrentObj(sender)) => {
                let result = state.export_current_obj();
                if let Err(_) = sender.send(result) {
                    log::error!("Failed to send ExportCurrentObj result");
                }
            }
            #[cfg(target_arch = "wasm32")]
            Event::UserEvent(UserEvent::ExportSplitObj(sender)) => {
                let result = state.export_current_obj_split();
                if let Err(_) = sender.send(result) {
                    log::error!("Failed to send ExportSplitObj result");
                }
            }
            #[cfg(target_arch = "wasm32")]
            Event::UserEvent(UserEvent::ViewClickGet(
                view_index,
                screen_x,
                screen_y,
                screen_z,
                sender,
            )) => {
                // Function-level comment: Compute view click result and send it back to JS via oneshot channel.
                let result = state.handle_view_click(view_index, screen_x, screen_y, screen_z);
                if let Err(_) = sender.send(result) {
                    log::error!("Failed to send ViewClickGet result for view {}", view_index);
                }
            }
            #[cfg(target_arch = "wasm32")]
            Event::UserEvent(UserEvent::GetPixelValue(view_index, screen_x, screen_y, sender)) => {
                let result = state.get_pixel_value_from_screen(view_index, screen_x, screen_y);
                if let Err(_) = sender.send(result) {
                    log::error!(
                        "Failed to send GetPixelValue result for view {}",
                        view_index
                    );
                }
            }
            #[cfg(target_arch = "wasm32")]
            Event::UserEvent(UserEvent::GetScreenCoordInMM(index, coord, sender)) => {
                // Function-level comment: Handle get_screen_coord_in_mm request and send result back via oneshot channel.
                let result = state.get_screen_coord_in_mm(index, coord);
                if let Err(_) = sender.send(result) {
                    log::error!(
                        "Failed to send GetScreenCoordInMM result for window {}",
                        index
                    );
                }
            }
            #[cfg(target_arch = "wasm32")]
            Event::UserEvent(UserEvent::GetWindowLevel(index, sender)) => {
                let result = state.get_window_level(index);
                if let Err(_) = sender.send(result) {
                    log::error!("Failed to send GetWindowLevel result for window {}", index);
                }
            }
            #[cfg(target_arch = "wasm32")]
            Event::UserEvent(UserEvent::GetBaseScreen(index, sender)) => {
                let result = state.get_base_screen(index);
                if let Err(_) = sender.send(result) {
                    log::error!("Failed to send GetBaseScreen result for window {}", index);
                }
            }
            #[cfg(target_arch = "wasm32")]
            Event::UserEvent(UserEvent::GetRotation(index, sender)) => {
                let result = state.get_rotation(index);
                if let Err(_) = sender.send(result) {
                    log::error!("Failed to send GetRotation result for window {}", index);
                }
            }
            #[cfg(target_arch = "wasm32")]
            Event::UserEvent(UserEvent::GetPan(index, sender)) => {
                let result = state.get_translate_in_screen_coord(index);
                if let Err(_) = sender.send(result) {
                    log::error!("Failed to send GetPan result for window {}", index);
                }
            }
            #[cfg(target_arch = "wasm32")]
            Event::UserEvent(UserEvent::GetObliqueNormal(index, sender)) => {
                let result = state.get_oblique_normal(index);
                if let Err(_) = sender.send(result) {
                    log::error!(
                        "Failed to send GetObliqueNormal result for window {}",
                        index
                    );
                }
            }
            #[cfg(target_arch = "wasm32")]
            Event::UserEvent(UserEvent::WorldCoordToScreen(index, coord, sender)) => {
                // Function-level comment: Handle world_coord_to_screen request and send result back via oneshot channel.
                let result = state.world_coord_to_screen(index, coord);
                if let Err(_) = sender.send(result) {
                    log::error!(
                        "Failed to send WorldCoordToScreen result for window {}",
                        index
                    );
                }
            }
            Event::WindowEvent {
                ref event,
                window_id,
            } if lifecycle.accepts_window(state.window().id(), window_id)
                && !state.input(event) =>
            {
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
                    } => {
                        lifecycle_event = Some(LifecycleEvent::Window {
                            id: window_id,
                            signal: WindowSignal::Close,
                        });
                    }
                    WindowEvent::Resized(physical_size) => {
                        log::info!("physical_size: {physical_size:?}");
                        lifecycle_event = Some(LifecycleEvent::Window {
                            id: window_id,
                            signal: WindowSignal::Resize(physical_size.width, physical_size.height),
                        });
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
                        state.set_render_mode(3, None, Some(0), None, 3);
                        state.set_oblique_rotation_radians(0, 0.0, 20.0, 0.0);
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
                        state.set_rotation_degrees(0, -90.0, 0.0);
                        state.set_rotation_degrees(0, 0.0, 90.0);
                        state.set_mesh_opacity(1.0);
                        state.set_scale(0, 2.0);
                        state.set_window_width(0, 300.0);
                        state.set_window_level(0, 300.0);
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
                        state.set_rotation_angle_degrees(0, -90.0, 90.0, 0.0);
                        let roi_point_min =
                            [-79.29739379882812, -50.16448974609375, -1241.34130859375];
                        let roi_point_max =
                            [86.64703369140625, 49.83551025390625, -978.4163208007812];
                        state.set_mesh_roi(
                            roi_point_min[0],
                            roi_point_min[1],
                            roi_point_min[2],
                            roi_point_max[0],
                            roi_point_max[1],
                            roi_point_max[2],
                        );
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
                        state.set_mip_mode(3, 0);
                        state.set_slab_thickness(3, 1.25);
                        state.set_scale(3, 2.0);
                        state.set_pan(3, 0.2, 0.2);
                        state.set_rotation_angle_degrees(3, 0.0, 180.0, 90.0);
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
                        state.set_pan(0, 0.0, 0.0);
                        state.set_window_width(0, 1500.0);
                        state.set_window_level(0, 400.0);
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
                    WindowEvent::Occluded(hidden) => {
                        lifecycle_event = Some(LifecycleEvent::Window {
                            id: window_id,
                            signal: WindowSignal::Occluded(*hidden),
                        });
                    }
                    WindowEvent::RedrawRequested => {
                        lifecycle_event = Some(LifecycleEvent::Window {
                            id: window_id,
                            signal: WindowSignal::Redraw,
                        });
                    }
                    _ => {}
                }
            }
            Event::Resumed => {
                lifecycle_event = Some(LifecycleEvent::Resumed);
            }
            Event::Suspended => lifecycle_event = Some(LifecycleEvent::Suspended),
            Event::AboutToWait => {
                lifecycle_event = Some(LifecycleEvent::AboutToWait);
            }
            _ => {}
        }
        if let Some(event) = lifecycle_event {
            lifecycle.handle(
                event,
                &mut WinitLifecycleHost {
                    state,
                    proxy,
                    target,
                },
                || clock.elapsed(),
            );
        }
    }
}

/// Borrowed platform adapter; the complete lifecycle sequence lives in ViewerLifecycle.
struct WinitLifecycleHost<'a> {
    state: &'a mut App,
    proxy: &'a EventLoopProxy<UserEvent>,
    target: &'a ActiveEventLoop,
}

impl LifecycleHost for WinitLifecycleHost<'_> {
    type WindowId = winit::window::WindowId;
    type Graphics = Graphics;
    type Volume = crate::data::ct_volume::CTVolume;

    fn active_window_id(&self) -> Self::WindowId {
        self.state.window().id()
    }
    fn surface_ready(&self) -> bool {
        self.state.can_render()
    }
    fn pending_work(&self) -> PendingWork {
        PendingWork {
            surface_ready: self.state.can_render(),
            dirty: self.state.needs_redraw(),
            animating: self.state.app_view.layout.is_animating(),
            recovery_in: self.state.app_view.layout.next_update_in(),
        }
    }
    fn invalidate_surface(&mut self) {
        self.state.invalidate_surface();
    }
    fn update(&mut self) {
        self.state.update();
    }
    fn render_frame(&mut self) -> crate::core::error::KeplerResult<super::frame::FrameOutcome> {
        self.state.render_frame()
    }
    fn swap_graphics(&mut self, graphics: Graphics) -> crate::core::error::KeplerResult<()> {
        self.state.try_swap_graphics(graphics)
    }
    fn attach_fault_transport(&mut self) {
        attach_fault_transport(self.state, self.proxy);
    }
    fn resize(&mut self, width: u32, height: u32) {
        self.state.resize(PhysicalSize { width, height });
    }
    fn enqueue_volume(&mut self, volume: Self::Volume) {
        self.proxy
            .send_event(UserEvent::LoadDataFromCTVolume(volume))
            .unwrap();
    }
    fn clear_views(&mut self) {
        self.state.app_view.layout.remove_all();
    }
    fn request_redraw(&mut self) {
        self.state.window().request_redraw();
    }
    fn wait(&mut self, delay: Option<std::time::Duration>) {
        // Winit chooses its native/web clock internally; no native Instant on WASM.
        self.target
            .set_control_flow(delay.map_or(ControlFlow::Wait, ControlFlow::wait_duration));
    }
    fn exit(&mut self) {
        self.target.exit();
    }
}
