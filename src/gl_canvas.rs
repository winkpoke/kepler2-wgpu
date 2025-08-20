use winit::event_loop::EventLoopProxy;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;


#[derive(Debug)]
pub enum UserEvent {
    SetSliceSpeed(usize, f32),
    SetWindowLevel(usize, f32),
    SetWindowWidth(usize, f32),
    SetSlice(usize, f32),
    SetScale(usize, f32),
    SetTranslate(usize, f32, f32, f32),  // translate in 3D space
    // ... add more events as needed
}

// #[macro_export]
// macro_rules! impl_user_event_senders_for_glcanvas {
//     (
//         $( $fn_name:ident => $variant:ident($arg:ident : $arg_ty:ty) ),* $(,)?
//     ) => {
//         #[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
//         impl GLCanvas {
//             $(
//                 /// Sends a `UserEvent::$variant` targeted to a specific window index.
//                 pub fn $fn_name(&self, index: usize, $arg: $arg_ty) {
//                     if let Err(e) = self.proxy.send_event(UserEvent::$variant(index, $arg)) {
//                         log::error!("Failed to send {} event for window {}: {:?}", stringify!($variant), index, e);
//                     } else {
//                         log::info!("Sent {} event for window {}: {:?}", stringify!($variant), index, $arg);
//                     }
//                 }
//             )*
//         }
//     };
// }

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
pub struct GLCanvas {
    pub(crate) proxy: EventLoopProxy<UserEvent>,
}

impl_user_event_senders_for_glcanvas! {
    set_slice_speed => SetSliceSpeed(speed: f32),
    set_window_level => SetWindowLevel(window_level: f32),
    set_window_width => SetWindowWidth(window_width: f32),
    set_slice => SetSlice(slice: f32),
    set_scale => SetScale(scale: f32),
    set_translate => SetTranslate(dx: f32, dy: f32, dz: f32),
}