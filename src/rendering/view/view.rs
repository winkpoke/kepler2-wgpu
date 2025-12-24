#![allow(dead_code)]

use std::any::Any;
use super::Renderable;
use crate::core::coord::Base;
use crate::core::geometry::GeometryBuilder;
// use crate::rendering::MprView;
use crate::core::{WindowLevel, error::KeplerResult};
use crate::CTVolume;

/// State snapshot for preserving view configuration during mode switches.
/// 
/// This structure captures all essential MPR view parameters that need to be restored
/// when transitioning between different view modes (e.g., mesh to MPR). It ensures
/// that users don't lose their current viewing configuration when switching between
/// different visualization modes.
/// 
/// ## Medical Imaging Context
/// 
/// In medical imaging workflows, users often need to switch between different
/// visualization modes while maintaining their current viewing parameters:
/// - Window/level settings for optimal tissue contrast
/// - Current slice position for anatomical reference
/// - Zoom and pan settings for detailed examination
/// - View positioning and dimensions
#[derive(Debug, Clone)]
pub struct ViewState {
    /// Window level (center) for CT image display - controls brightness
    pub window_level: f32,
    /// Window width for CT image display - controls contrast
    pub window_width: f32,
    /// Bias offset applied to window level for fine-tuning
    pub bias: f32,
    /// Current slice position in millimeters along the view normal
    pub slice_mm: f32,
    /// Current zoom scale factor (1.0 = original size)
    pub scale: f32,
    /// Translation in screen coordinate space [x, y, z] - used for panning
    pub translate_in_screen_coord: [f32; 3],
    /// View position on screen (top-left corner) in pixels
    pub position: (i32, i32),
    /// View dimensions (width, height) in pixels
    pub dimensions: (u32, u32),
}

impl ViewState {
    /// Create a new ViewState with default values for medical imaging.
    /// 
    /// Uses standard CT window/level settings and neutral positioning that work
    /// well for most medical imaging scenarios. The default window/level values
    /// are optimized for soft tissue visualization.
    pub fn new() -> Self {
        Self {
            window_level: 40.0,    // Standard CT soft tissue window level (HU)
            window_width: 400.0,   // Standard CT soft tissue window width (HU)
            bias: 0.0,             // No bias offset initially
            slice_mm: 0.0,         // Start at center slice
            scale: 1.0,            // No zoom initially
            translate_in_screen_coord: [0.0, 0.0, 0.0],  // No screen-space panning
            position: (0, 0),      // Top-left corner
            dimensions: (512, 512), // Standard medical imaging size
        }
    }

    /// Create a WindowLevel instance from the state's window/level parameters.
    /// 
    pub fn create_window_level(&self) -> KeplerResult<WindowLevel> {
        let mut window_level = WindowLevel::new();
        window_level.set_window_width(self.window_width)?;
        window_level.set_window_level(self.window_level)?;
        window_level.set_bias(self.bias)?;
        Ok(window_level)
    }

    /// Validate that the view state contains reasonable values.
    /// 
    /// Ensures window width is positive, scale is within reasonable bounds, and dimensions are valid.
    /// This prevents invalid states from being restored and causing rendering issues.
    /// 
    /// ## Validation Rules
    /// 
    /// - Window width must be positive (required for proper CT display)
    /// - Scale must be positive and less than 100x (prevents extreme zoom levels)
    /// - Dimensions must be non-zero (required for valid viewport)
    pub fn is_valid(&self) -> bool {
        self.window_width > 0.0 
            && self.scale > 0.0 
            && self.scale < 100.0  // Reasonable scale limit to prevent extreme zoom
            && self.dimensions.0 > 0 
            && self.dimensions.1 > 0
    }
}

impl Default for ViewState {
    fn default() -> Self {
        Self::new()
    }
}

/// Core trait for all renderable views in the medical imaging system.
/// 
/// This trait extends the `Renderable` trait with view-specific functionality
/// including position management, resizing, and type introspection. All views
/// in the system must implement this trait to be managed by the layout system.
/// 
/// ## Design Philosophy
/// 
/// The trait is designed to be minimal but complete, providing only the essential
/// operations needed for view management while allowing concrete implementations
/// to add specialized functionality.
pub trait View: Renderable + Any {
    // type ViewType;
    /// Get the current position of the view on screen (top-left corner in pixels)
    fn position(&self) -> (i32, i32);
    
    /// Get the current dimensions of the view (width, height in pixels)
    fn dimensions(&self) -> (u32, u32);
    
    /// Move the view to a new position on screen
    fn move_to(&mut self, pos: (i32, i32));
    
    /// Resize the view to new dimensions
    fn resize(&mut self, dim: (u32, u32));
    
    /// Get a reference to this view as Any for type introspection
    fn as_any(&self) -> &dyn Any;
    
    /// Get a mutable reference to this view as Any for type introspection
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

/// Enhanced View trait with state management capabilities.
/// 
/// Allows views to save their current state and restore from a saved state,
/// enabling seamless transitions between different view modes. This is particularly
/// important in medical imaging where users need to maintain their viewing
/// configuration when switching between different visualization modes.
/// 
/// ## Use Cases
/// 
/// - Switching between 2D MPR and 3D mesh visualization
/// - Temporarily switching to a different view orientation
/// - Saving user preferences for later sessions
/// - Implementing undo/redo functionality for view changes
pub trait StatefulView: View {
    /// Save the current view state for later restoration.
    /// 
    /// Returns None if the view doesn't support state saving or if the current
    /// state is invalid. Implementations should validate the state before saving
    /// to ensure it can be successfully restored later.
    fn save_state(&self) -> Option<ViewState>;
    
    /// Restore view state from a previously saved snapshot.
    /// 
    /// Returns true if restoration was successful, false if the state was invalid
    /// or incompatible with the current view. Implementations should validate
    /// the incoming state and handle any necessary coordinate transformations.
    fn restore_state(&mut self, state: &ViewState) -> bool;
    
    /// Get a string identifier for the view type.
    /// 
    /// Used for type checking and debugging during view transitions. This helps
    /// ensure that states are only restored to compatible view types.
    fn view_type(&self) -> &'static str;
}

/// Anatomical orientation for MPR views.
/// 
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Orientation {
    /// Custom orientation not aligned with standard anatomical planes
    Oblique,
    /// Vertical slices from side to side (left-right separation)
    Sagittal,
    /// Vertical slices from front to back (anterior-posterior separation)
    Coronal,
    /// Horizontal slices from top to bottom (superior-inferior separation)
    Transverse,
}

/// Standard anatomical orientations in order of common usage
pub const ALL_ORIENTATIONS: [Orientation; 4] = [
    Orientation::Transverse,  // Most commonly used in CT
    Orientation::Coronal,
    Orientation::Sagittal,
    Orientation::Oblique,     // Least commonly used
];

impl Orientation {
    /// Build the coordinate system base for this orientation.
    /// 
    /// Creates the appropriate coordinate transformation matrix for the given
    /// anatomical orientation, taking into account the volume's spatial properties
    /// and DICOM coordinate system conventions.
    pub fn build_base(&self, vol: &CTVolume) -> Base {
        match self {
            Orientation::Oblique => GeometryBuilder::build_oblique_base(vol),
            Orientation::Sagittal => GeometryBuilder::build_sagittal_base(vol),
            Orientation::Coronal => GeometryBuilder::build_coronal_base(vol),
            Orientation::Transverse => GeometryBuilder::build_transverse_base(vol),
        }
    }
}