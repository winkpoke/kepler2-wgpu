use winit::event_loop::EventLoopProxy;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;


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