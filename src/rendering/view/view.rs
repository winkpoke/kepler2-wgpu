//! # Medical Imaging View System
//! 
//! This module provides the core view system for medical imaging visualization in the Kepler WGPU framework.
//! It supports Multi-Planar Reconstruction (MPR) views with different anatomical orientations and provides
//! state management capabilities for seamless view transitions.
//! 
//! ## Key Components
//! 
//! - **View Traits**: Core interfaces for renderable views with position and dimension management
//! - **StatefulView**: Enhanced views that can save and restore their configuration state
//! - **ViewFactory**: Factory pattern for consistent view creation
//! - **MPRView**: Specialized interface for medical imaging views with window/level controls
//! - **GenericMPRView**: Concrete implementation supporting all anatomical orientations
//! - **ViewState**: State snapshot structure for preserving view configurations
//! 
//! ## Medical Imaging Context
//! 
//! The view system is designed specifically for medical imaging applications:
//! - Supports standard anatomical orientations (Transverse, Coronal, Sagittal, Oblique)
//! - Provides window/level controls for CT image display
//! - Handles slice navigation in millimeter units
//! - Maintains accurate spatial relationships and measurements
//! 
//! ## Usage Example
//! 
//! Creating an MPR view for medical imaging:
//! - Use `GenericMPRView::new()` with appropriate parameters
//! - Set orientation (Transverse, Coronal, Sagittal, or Oblique)
//! - Configure scale, translation, position, and dimensions
//! - Use `StatefulView` trait methods to save/restore view state
//! - Use `MPRView` trait methods to control window/level and slice position

#![allow(dead_code)]

use std::any::Any;
use super::Renderable;
use crate::core::coord::Base;
use crate::core::geometry::GeometryBuilder;
// use crate::rendering::MprView;
use crate::core::{WindowLevel, error::KeplerResult};
use crate::rendering::MprView;
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
    /// This method creates a properly configured WindowLevel struct using the
    /// window_width, window_level, and bias values from the state. The resulting
    /// WindowLevel will be marked as dirty to ensure GPU updates occur.
    /// 
    /// # Returns
    /// * `KeplerResult<WindowLevel>` - Success with configured WindowLevel or validation error
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

    // Attempt to cast this view to an MPRView for medical imaging operations.
    // Returns None if this view doesn't support MPR functionality.
    // fn as_mpr(&mut self) -> Option<&mut Self::ViewType> {
    //     if let Some(view) = self.as_any_mut().downcast_mut::<Self::ViewType>() {
    //         Some(view)
    //     } else {
    //         None
    //     }
    // }
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

/// Factory trait for creating different types of views.
/// 
/// Centralizes view creation logic and provides a consistent interface for
/// creating views with proper initialization parameters. This pattern ensures
/// that all views are created with the correct dependencies and configuration.
/// 
/// ## Benefits
/// 
/// - Consistent view initialization across the application
/// - Centralized dependency injection for view creation
/// - Easy testing through mock factory implementations
/// - Type-safe view creation with proper error handling
pub trait ViewFactory {
    /// Create a new mesh view with specified position and dimensions.
    /// 
    /// Returns a boxed View trait object ready for rendering 3D mesh data.
    /// The view will be configured for 3D visualization with appropriate
    /// camera settings and rendering pipeline.
    fn create_mesh_view(
        &self, 
        pos: (i32, i32), 
        size: (u32, u32)
    ) -> Result<Box<dyn View>, Box<dyn std::error::Error>>;
    
    /// Create a new MPR view with volume data and orientation.
    /// 
    /// Returns a boxed View trait object configured for medical imaging display.
    /// The view will be set up with appropriate shaders, uniforms, and geometry
    /// for the specified anatomical orientation.
    /// 
    /// ## Parameters
    /// 
    /// - `vol`: CT volume data containing the medical imaging dataset
    /// - `orientation`: Anatomical orientation (Transverse, Coronal, Sagittal, Oblique)
    /// - `pos`: Initial position on screen
    /// - `size`: Initial dimensions of the view
    fn create_mpr_view(
        &self, 
        vol: &CTVolume, 
        orientation: Orientation, 
        pos: (i32, i32), 
        size: (u32, u32)
    ) -> Result<Box<dyn View>, Box<dyn std::error::Error>>;
}

/// Specialized trait for Multi-Planar Reconstruction (MPR) views in medical imaging.
/// 
/// Provides medical imaging-specific functionality including window/level controls,
/// slice navigation, and spatial transformations. This trait extends the base View
/// trait with operations specific to medical image visualization.
/// 
/// ## Medical Imaging Concepts
/// 
/// - **Window/Level**: Controls image brightness and contrast for optimal tissue visualization
/// - **Slice Navigation**: Moves through the volume in millimeter units along the view normal
/// - **Scale/Pan**: Provides zoom and pan functionality for detailed examination
/// - **Coordinate Systems**: Handles both screen-space and medical-space coordinates
// pub trait MPRView: View {
//     // === Setters for view parameters ===
    
//     /// Set the window level (brightness control) for CT image display
//     fn set_window_level(&mut self, window_level: f32);
    
//     /// Set the window width (contrast control) for CT image display
//     fn set_window_width(&mut self, window_width: f32);
    
//     /// Set the current slice position in millimeters along the view normal
//     fn set_slice_mm(&mut self, z: f32);
    
//     /// Set the zoom scale factor (1.0 = original size)
//     fn set_scale(&mut self, scale: f32);
    
//     /// Set translation in view/model coordinate space
//     fn set_translate(&mut self, translate: [f32; 3]);
    
//     /// Set translation in screen coordinate space (for panning)
//     fn set_translate_in_screen_coord(&mut self, translate: [f32; 3]);
    
//     /// Pan the view in screen space by the specified amounts
//     fn set_pan(&mut self, x: f32, y: f32);
    
//     /// Pan the view by the specified amounts in millimeters
//     fn set_pan_mm(&mut self, x_mm: f32, y_mm: f32);
    
//     // === Getters for view parameters ===
    
//     /// Returns current window level used by the fragment shader
//     fn get_window_level(&self) -> f32;
    
//     /// Returns current window width used by the fragment shader
//     fn get_window_width(&self) -> f32;
    
//     /// Returns current slice position in millimeters along view normal
//     fn get_slice_mm(&self) -> f32;
    
//     /// Returns current scale factor applied in screen space
//     fn get_scale(&self) -> f32;
    
//     /// Returns current pan/translation in screen coordinates [x, y, z]
//     fn get_translate_in_screen_coord(&self) -> [f32; 3];
    
//     /// Returns current translation in view/model coordinates [x, y, z]
//     fn get_translate(&self) -> [f32; 3];
// }

/// Anatomical orientation for MPR views.
/// 
/// Defines the standard anatomical orientations used in medical imaging.
/// Each orientation provides a different cross-sectional view of the patient's anatomy.
/// 
/// ## Medical Context
/// 
/// - **Transverse (Axial)**: Horizontal slices, looking from feet toward head
/// - **Coronal**: Vertical slices, looking from front to back
/// - **Sagittal**: Vertical slices, looking from side to side
/// - **Oblique**: Custom orientation, not aligned with standard anatomical planes
#[derive(Debug, Clone, Copy)]
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
    pub fn build_base(&self, vol: &CTVolume) -> Base<f32> {
        match self {
            Orientation::Oblique => GeometryBuilder::build_oblique_base(vol),
            Orientation::Sagittal => GeometryBuilder::build_sagittal_base(vol),
            Orientation::Coronal => GeometryBuilder::build_coronal_base(vol),
            Orientation::Transverse => GeometryBuilder::build_transverse_base(vol),
        }
    }

    /// Get the default slice navigation speed for this orientation.
    /// 
    /// Different orientations may have different optimal navigation speeds
    /// based on typical slice thickness and user interaction patterns.
    /// Transverse views typically have thicker slices and can use faster navigation.
    pub fn default_slice_speed(&self) -> f32 {
        match self {
            Orientation::Transverse => 0.006,  // Faster for thicker axial slices
            _ => 0.0005,  // Slower for thinner coronal/sagittal slices
        }
    }
}