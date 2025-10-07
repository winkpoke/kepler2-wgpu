use winit::event_loop::EventLoopProxy;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

use crate::{data::ct_volume::CTVolume, rendering::core::state::Graphics};


#[derive(Debug)]
pub enum UserEvent {
    SetSliceSpeed(usize, f32),
    SetWindowLevel(usize, f32),
    SetWindowWidth(usize, f32),
    SetSliceMM(usize, f32),
    SetScale(usize, f32),
    SetTranslateInScreenCoord(usize, f32, f32, f32),
    SetPan(usize, f32, f32), // pan in screen space
    SetPanMM(usize, f32, f32), // pan in mm
    SetTranslate(usize, f32, f32, f32),  // translate in 3D space
    LoadDataFromCTVolume(CTVolume), 
    Resize(u32, u32), // width, height
    Quit,
    SetWindowByDivId(String, CTVolume),
    GraphicsReady(Graphics, CTVolume),
    ClearLayout,
    /// Manually trigger shader reload by invalidating pipelines; pipelines will be lazily rebuilt on next render.
    ReloadShaders,
    /// Manually trigger pipeline cache invalidation without any other action.
    InvalidatePipelines,
    SetEnableMesh(bool),
    // ... add more events as needed
}

#[macro_export]
macro_rules! impl_user_event_senders_for_glcanvas {
    (
        $( $fn_name:ident => $variant:ident( $( $arg:ident : $arg_ty:ty ),* ) ),* $(,)?
    ) => {
        #[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
        impl GLCanvas {
            $(
                /// Sends a `UserEvent::$variant` targeted to a specific window index.
                pub fn $fn_name(&self, index: usize, $( $arg: $arg_ty ),* ) {
                    if let Err(e) = self.proxy.send_event(UserEvent::$variant(index, $( $arg ),*)) {
                        log::error!(
                            "Failed to send {} event for window {}: {:?}",
                            stringify!($variant),
                            index,
                            e
                        );
                    } else {
                        log::info!(
                            "Sent {} event for window {}: {:?}",
                            stringify!($variant),
                            index,
                            ( $( $arg ),* )
                        );
                    }
                }
            )*
        }
    };
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
impl GLCanvas {
    pub fn load_data_from_ct_volume(&self, volume: &CTVolume) {
        if let Err(e) = self.proxy.send_event(UserEvent::LoadDataFromCTVolume(volume.clone())) {
            log::error!("Failed to send LoadDataFromCTVolume event {:?}", e);
        } else {
            log::info!("Sent LoadDataFromCTVolume event");
        }
    }

    pub fn resize(&self, width: u32, height: u32) {
        if let Err(e) = self.proxy.send_event(UserEvent::Resize(width, height)) {
            log::error!("Failed to send Resize event: {:?}", e);
        } else {
            log::info!("Sent Resize event: width={}, height={}", width, height);
        }
    }

    pub fn quit(&self) {
        if let Err(e) = self.proxy.send_event(UserEvent::Quit) {
            log::error!("Failed to send Quit event: {:?}", e);
        } else {
            log::info!("Sent Quit event");
        }
    }

    pub fn set_window_by_div_id(&self, div_id: String, volume: &CTVolume) {
        if let Err(e) = self.proxy.send_event(UserEvent::SetWindowByDivId(div_id.clone(), volume.clone())) {
            log::error!("Failed to send SetWindowByDivId event for div_id {}: {:?}", div_id, e);
        } else {
            log::info!("Sent SetWindowByDivId event for div_id {}", div_id);
        }
    }
    pub fn clear_layout(&self) {
        if let Err(e) = self.proxy.send_event(UserEvent::ClearLayout) {
            log::error!("Failed to send ClearLayout event: {:?}", e);
        } else {
            log::info!("Sent ClearLayout event");
        }
    }

    /// Sends a ReloadShaders event which will invalidate pipelines; they will be recreated as needed.
    pub fn reload_shaders(&self) {
        if let Err(e) = self.proxy.send_event(UserEvent::ReloadShaders) {
            log::error!("Failed to send ReloadShaders event: {:?}", e);
        } else {
            log::info!("Sent ReloadShaders event");
        }
    }

    /// Sends an InvalidatePipelines event without any shader changes.
    pub fn invalidate_pipelines(&self) {
        if let Err(e) = self.proxy.send_event(UserEvent::InvalidatePipelines) {
            log::error!("Failed to send InvalidatePipelines event: {:?}", e);
        } else {
            log::info!("Sent InvalidatePipelines event");
        }
    }

    pub fn enable_mesh(&self, enabled: bool) {
        if let Err(e) = self.proxy.send_event(UserEvent::SetEnableMesh(enabled)) {
            log::error!("Failed to send SetEnableMesh event: {:?}", e);
        } else {
            log::info!("Sent SetEnableMesh event: enabled={}", enabled);
        }
    }
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub struct GLCanvas {
    pub(crate) proxy: EventLoopProxy<UserEvent>,
}

impl_user_event_senders_for_glcanvas! {
    set_slice_speed => SetSliceSpeed(speed: f32),
    set_window_level => SetWindowLevel(window_level: f32),
    set_window_width => SetWindowWidth(window_width: f32),
    set_slice_mm => SetSliceMM(slice: f32),
    set_scale => SetScale(scale: f32),
    set_translate => SetTranslate(dx: f32, dy: f32, dz: f32),
    set_translate_in_screen_coord => SetTranslateInScreenCoord(x: f32, y: f32, z: f32),
    set_pan => SetPan(dx: f32, dy: f32),
    set_pan_mm => SetPanMM(dx_mm: f32, dy_mm: f32),
}