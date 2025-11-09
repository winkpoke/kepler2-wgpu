//! AppView: centralizes layout and view factory ownership
//!
//! Minimal AppView that owns DynamicLayout and DefaultViewFactory.
//! State will hold AppView and forward calls, keeping existing render loop intact.

use crate::rendering::view::{DynamicLayout, DefaultViewFactory};
use crate::rendering::LayoutContainer;

/// AppView holds layout and view factory for building and arranging views.
pub struct AppView {
    pub(crate) layout: DynamicLayout,
    pub(crate) view_factory: DefaultViewFactory,
}

impl AppView {
    /// Construct a new AppView from a layout and a view factory.
    ///
    /// Function-level comment: This constructor enables State to transfer ownership of
    /// layout and factory with minimal changes to existing code.
    pub fn new(layout: DynamicLayout, view_factory: DefaultViewFactory) -> Self {
        Self { layout, view_factory }
    }

    /// Resize the layout to match new parent dimensions.
    ///
    /// Function-level comment: Convenience wrapper for layout.resize.
    pub fn resize(&mut self, dim: (u32, u32)) {
        // Call through the LayoutContainer trait to ensure method resolution in all scopes.
        LayoutContainer::resize(&mut self.layout, dim);
    }
}