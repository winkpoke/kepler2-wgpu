use std::any::Any;
use super::Renderable;

pub trait View: Renderable + Any {
    fn position(&self) -> (i32, i32);
    fn dimensions(&self) -> (u32, u32);
    fn move_to(&mut self, pos: (i32, i32));
    fn resize(&mut self, dim: (u32, u32));
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;

    fn as_mpr(&mut self) -> Option<&mut dyn MPRView> {
        None
    }
}

pub trait MPRView: View {
    // fn set_slice(&mut self, slice: u32);
    fn set_window_level(&mut self, window_level: f32);
}