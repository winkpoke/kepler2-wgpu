//! AppView: centralizes layout and view factory ownership
//!
//! Minimal AppView that owns DynamicLayout and DefaultViewFactory.
//! State will hold AppView and forward calls, keeping existing render loop intact.

use crate::rendering::view::mesh::mesh::Mesh;
use crate::rendering::view::render_content::RenderContent;
use crate::rendering::view::ViewFactory;
use crate::rendering::view::{
    DefaultViewFactory, DynamicLayout, MipView, MprView, Orientation, View, ViewState,
    ALL_ORIENTATIONS,
};
use crate::rendering::view::mip::MipConfig;
use crate::rendering::view::mesh::mesh_view::MeshView;
use crate::rendering::StatefulView;
use crate::rendering::{GridLayout, LayoutContainer, OneCellLayout};
use crate::CTVolume;
use std::sync::Arc;
use glam::Mat4;

/// Encapsulated state for a view, including its orientation and rendering parameters.
#[derive(Debug, Clone)]
pub enum CapturedViewState {
    Mpr {
        view_index: usize,
        orientation: Orientation,
        state: ViewState,
    },
    Mip {
        config: MipConfig,
        scale: f32,
        pan: [f32; 3],
        rotation_radians: [f32; 3],
    },
    Mesh {
        rotation: Mat4,
        scale: f32,
        pan: [f32; 3],
        opacity: f32,
    },
}

/// AppView holds layout and view factory for building and arranging views.
pub struct AppView {
    pub(crate) layout: DynamicLayout,
    pub(crate) view_factory: DefaultViewFactory,
    pub(crate) saved_states: Vec<CapturedViewState>,
}

impl AppView {
    /// Construct a new AppView from a layout and a view factory.
    ///
    /// Function-level comment: This constructor enables State to transfer ownership of
    /// layout and factory with minimal changes to existing code.
    pub fn new(layout: DynamicLayout, view_factory: DefaultViewFactory) -> Self {
        Self {
            layout,
            view_factory,
            saved_states: Vec::new(),
        }
    }

    /// Capture and save the state of all compatible views (MPR, MIP, Mesh).
    ///
    /// Function-level comment: Iterates over views, identifies their type, and saves their state
    /// into the internal saved_states buffer. This enables restoring exact view configurations
    /// after layout switches (e.g., Single <-> Multi-view).
    pub fn save_view_states(&mut self) {
        self.saved_states = self.layout
            .views()
            .iter()
            .enumerate()
            .filter_map(|(i, v)| {
                if let Some(mpr_view) = v.as_any().downcast_ref::<MprView>() {
                    let orientation = mpr_view.get_orientation();
                    mpr_view.save_state().map(|state| CapturedViewState::Mpr {
                        view_index: i,
                        orientation: *orientation,
                        state,
                    })
                } else if let Some(mip_view) = v.as_any().downcast_ref::<MipView>() {
                    Some(CapturedViewState::Mip {
                        config: mip_view.config().clone(),
                        scale: mip_view.get_scale(),
                        pan: mip_view.get_pan(),
                        rotation_radians: mip_view.get_rotation_radians(),
                    })
                } else if let Some(mesh_view) = v.as_any().downcast_ref::<MeshView>() {
                    Some(CapturedViewState::Mesh {
                        rotation: mesh_view.get_rotation(),
                        scale: mesh_view.get_scale_factor(),
                        pan: mesh_view.get_pan(),
                        opacity: mesh_view.get_opacity(),
                    })
                } else {
                    None
                }
            })
            .collect();
    }

    /// Restore previously captured view states to matching views.
    ///
    /// Function-level comment: Matches internally saved states to current views by type and orientation
    /// (for MPR) and applies the saved parameters.
    pub fn restore_view_states(&mut self) {
        // Clone states to avoid borrowing self.saved_states while mutating self.layout
        let states = self.saved_states.clone();
        
        for (i, view) in self.layout.views_mut().iter_mut().enumerate() {
            if let Some(mpr_view) = view.as_any_mut().downcast_mut::<MprView>() {
                if let Some(CapturedViewState::Mpr { state, .. }) = states
                    .iter()
                    .find(|s| matches!(s, CapturedViewState::Mpr { view_index, orientation: o, .. } 
                        if *view_index == i && *o == *mpr_view.get_orientation()))
                {
                    mpr_view.restore_state(state);
                }
            }
            // Handle MIP Views
            else if let Some(mip_view) = view.as_any_mut().downcast_mut::<MipView>() {
                if let Some(CapturedViewState::Mip { config, scale, pan, rotation_radians }) = states
                    .iter()
                    .find(|s| matches!(s, CapturedViewState::Mip { .. }))
                {
                    mip_view.set_mip_mode(config.mip_mode);
                    mip_view.set_slab_thickness(config.slab_thickness);
                    mip_view.set_scale(*scale);
                    mip_view.set_pan(pan[0], pan[1]); // Note: MIP pan is 3D stored but 2D set currently, Z is 0.0 usually.
                    mip_view.set_rotation_radians(rotation_radians[0],rotation_radians[1],rotation_radians[2]);
                }
            }
            // Handle Mesh Views
            else if let Some(mesh_view) = view.as_any_mut().downcast_mut::<MeshView>() {
                if let Some(CapturedViewState::Mesh { rotation, scale, pan, opacity }) = states
                    .iter()
                    .find(|s| matches!(s, CapturedViewState::Mesh { .. }))
                {
                    mesh_view.set_rotation(*rotation);
                    mesh_view.set_scale_factor(*scale);
                    mesh_view.set_pan(pan[0],pan[1]);
                    mesh_view.set_opacity(*opacity);
                }
            }
        }
    }

    /// Resize the layout to match new parent dimensions.
    ///
    /// Function-level comment: Convenience wrapper for layout.resize.
    pub fn resize(&mut self, dim: (u32, u32)) {
        // Call through the LayoutContainer trait to ensure method resolution in all scopes.
        LayoutContainer::resize(&mut self.layout, dim);
    }

    /// Add a view to the layout with automatic positioning and sizing.
    ///
    /// Function-level comment: Convenience wrapper to centralize view addition via AppView.
    pub fn add_view(&mut self, view: Box<dyn View>) {
        LayoutContainer::add_view(&mut self.layout, view);
    }

    /// Remove all views from the layout.
    ///
    /// Function-level comment: Clears the layout's view registry.
    pub fn remove_all(&mut self) {
        LayoutContainer::remove_all(&mut self.layout);
    }

    /// Get the total number of views managed by the layout.
    ///
    /// Function-level comment: Exposes count for UI and orchestration logic.
    pub fn view_count(&self) -> usize {
        LayoutContainer::view_count(&self.layout)
    }

    /// Get a view by index if it exists.
    ///
    /// Function-level comment: Read-only access to a single view for orchestration.
    pub fn get_view_by_index(&self, index: usize) -> Option<&Box<dyn View>> {
        LayoutContainer::get_view_by_index(&self.layout, index)
    }

    /// Get a mutable reference to a view by index if it exists.
    ///
    /// Function-level comment: Mutable access to a single view for interaction updates.
    pub fn get_view_mut(&mut self, index: usize) -> Option<&mut Box<dyn View>> {
        LayoutContainer::get_view_mut(&mut self.layout, index)
    }

    /// Check if the view at an index is of a specific type.
    ///
    /// Function-level comment: Generic wrapper around LayoutContainer::is_view_type.
    pub fn is_view_type<V: View + 'static>(&self, index: usize) -> bool {
        LayoutContainer::is_view_type::<V>(&self.layout, index)
    }

    /// Replace a view at the given index with a new one, returning the old view if present.
    ///
    /// Function-level comment: Wrapper around LayoutContainer::replace_view_at for lifecycle management.
    pub fn replace_view_at(
        &mut self,
        index: usize,
        new_view: Box<dyn View>,
    ) -> Option<Box<dyn View>> {
        LayoutContainer::replace_view_at(&mut self.layout, index, new_view)
    }

    /// Switch layout strategy to a single-cell layout (OneCellLayout).
    ///
    /// Function-level comment: Centralizes strategy changes through AppView for active-view workflows.
    pub fn set_one_cell_layout(&mut self) {
        self.layout.set_strategy(Box::new(OneCellLayout {
            rows: 1,
            cols: 1,
            spacing: 0,
        }));
    }

    /// Switch layout strategy to a grid layout.
    ///
    /// Function-level comment: Exposes grid layout configuration via AppView.
    pub fn set_grid_layout(&mut self, rows: u32, cols: u32, spacing: u32) {
        self.layout.set_strategy(Box::new(GridLayout {
            rows,
            cols,
            spacing,
        }));
    }

    /// Returns true if the current layout strategy is OneCellLayout.
    ///
    /// Function-level comment: Helper to gate active-view-specific operations.
    pub fn is_one_cell_layout(&self) -> bool {
        self.layout.strategy_id() == "OneCellLayout"
    }

    /// Create and add an MPR view for a given volume and orientation.
    ///
    /// Function-level comment: Uses DefaultViewFactory and routes addition through AppView.
    pub fn add_mpr_view(
        &mut self,
        vol: &CTVolume,
        orientation: Orientation,
        pos: (i32, i32),
        size: (u32, u32),
    ) -> Result<(), Box<dyn std::error::Error>> {
        let view = self
            .view_factory
            .create_mpr_view(vol, orientation, pos, size)?;
        LayoutContainer::add_view(&mut self.layout, view);
        Ok(())
    }

    /// Create and add an MIP view for a given volume.
    ///
    /// Function-level comment: Uses DefaultViewFactory and routes addition through AppView.
    pub fn add_mip_view(
        &mut self,
        vol: &CTVolume,
        pos: (i32, i32),
        size: (u32, u32),
    ) -> Result<(), Box<dyn std::error::Error>> {
        let view = self.view_factory.create_mip_view(vol, pos, size)?;
        LayoutContainer::add_view(&mut self.layout, view);
        Ok(())
    }

    /// Create and add a Mesh view for a given mesh.
    ///
    /// Function-level comment: Uses DefaultViewFactory and routes addition through AppView.
    pub fn add_mesh_view(
        &mut self,
        mesh: &Mesh,
        pos: (i32, i32),
        size: (u32, u32),
    ) -> Result<(), Box<dyn std::error::Error>> {
        let view = self.view_factory.create_mesh_view(mesh, pos, size)?;
        LayoutContainer::add_view(&mut self.layout, view);
        Ok(())
    }

    /// Create and add an MPR view using shared RenderContent for zero-copy reuse.
    ///
    /// Function-level comment: Uses DefaultViewFactory to reuse existing volume textures.
    pub fn add_mpr_view_with_content(
        &mut self,
        render_content: Arc<RenderContent>,
        vol: &CTVolume,
        orientation: Orientation,
        pos: (i32, i32),
        size: (u32, u32),
    ) -> Result<(), Box<dyn std::error::Error>> {
        let view = self.view_factory.create_mpr_view_with_content(
            render_content,
            vol,
            orientation,
            pos,
            size,
        )?;
        LayoutContainer::add_view(&mut self.layout, view);
        Ok(())
    }

    /// Create and add an MIP view using shared RenderContent for zero-copy reuse.
    ///
    /// Function-level comment: Uses DefaultViewFactory to reuse existing volume textures.
    pub fn add_mip_view_with_content(
        &mut self,
        render_content: Arc<RenderContent>,
        pos: (i32, i32),
        size: (u32, u32),
    ) -> Result<(), Box<dyn std::error::Error>> {
        let view = self
            .view_factory
            .create_mip_view_with_content(render_content, pos, size)?;
        LayoutContainer::add_view(&mut self.layout, view);
        Ok(())
    }

    /// Reset the layout to the default 4-MPR view configuration.
    ///
    /// Function-level comment: Rebuilds the standard 2x2 layout with Transverse, Coronal, Sagittal, and Transverse views.
    pub fn reset_to_default_mpr_layout(
        &mut self,
        texture: Arc<RenderContent>,
        vol: &CTVolume,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.remove_all();
        // Default layout uses orientations: Transverse, Coronal, Sagittal, Transverse
        // Corresponds to ALL_ORIENTATIONS indices [0, 1, 2, 0]
        let orientations = [
            ALL_ORIENTATIONS[0],
            ALL_ORIENTATIONS[1],
            ALL_ORIENTATIONS[2],
            ALL_ORIENTATIONS[0],
        ];

        for orientation in orientations.iter() {
            let view = self.view_factory.create_mpr_view_with_content(
                texture.clone(),
                vol,
                *orientation,
                (0, 0),
                (0, 0),
            )?;
            LayoutContainer::add_view(&mut self.layout, view);
        }
        Ok(())
    }

    /// Configure the layout (4 MPR views + optional MIP/Mesh).
    ///
    /// Function-level comment: Sets up a 2x2 layout with 3 MPR views and one special view (MIP or Mesh) in the 4th slot.
    pub fn configure_mesh_layout(
        &mut self,
        texture: Arc<RenderContent>,
        vol: &CTVolume,
        indices: [usize; 4],
        mip: Option<usize>,
        orientation_index: usize,
        mesh_index: Option<usize>,
        cached_mesh: Option<crate::mesh::mesh::Mesh>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.remove_all();

        // Add 4 MPR views based on indices
        for &index in indices.iter() {
            let orientation = ALL_ORIENTATIONS[index];
            let view = self.view_factory.create_mpr_view_with_content(
                texture.clone(),
                vol,
                orientation,
                (0, 0),
                (0, 0),
            )?;
            LayoutContainer::add_view(&mut self.layout, view);
        }

        if mip.is_some() {
            let mip_view =
                self.view_factory
                    .create_mip_view_with_content(texture.clone(), (0, 0), (0, 0))?;
            LayoutContainer::replace_view_at(&mut self.layout, mip.unwrap(), mip_view);
            let _ = self.set_mip_mode(mip.unwrap(), orientation_index as u32);
        }

        if mesh_index.is_some() {
            let mesh_view = self.view_factory.create_mesh_view_with_content(
                texture.clone(),
                &cached_mesh.unwrap(),
                (0, 0),
                (0, 0),
            )?;
            LayoutContainer::replace_view_at(&mut self.layout, mesh_index.unwrap(), mesh_view);
        }

        Ok(())
    }

    /// Switch to a single-cell layout and display the requested view type.
    ///
    /// Function-level comment: Configures a single large view for detailed inspection (MPR, MIP).
    pub fn set_layout_mode_single(
        &mut self,
        texture: Arc<RenderContent>,
        vol: &CTVolume,
        mode: usize,
        orientation_index: usize,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.set_one_cell_layout();
        self.remove_all();

        match mode {
            0 => {
                // MPR
                let orientation = ALL_ORIENTATIONS[orientation_index];
                let view = self.view_factory.create_mpr_view_with_content(
                    texture.clone(),
                    vol,
                    orientation,
                    (0, 0),
                    (0, 0),
                )?;
                LayoutContainer::add_view(&mut self.layout, view);
            }
            1 => {
                // MIP
                let mip_view = self.view_factory.create_mip_view_with_content(
                    texture.clone(),
                    (0, 0),
                    (0, 0),
                )?;
                LayoutContainer::add_view(&mut self.layout, mip_view);
                let _ = self.set_mip_mode(0, orientation_index as u32);
            }
            _ => {
                // Default to MPR for unsupported modes
                let orientation = ALL_ORIENTATIONS[orientation_index];
                let view = self.view_factory.create_mpr_view_with_content(
                    texture.clone(),
                    vol,
                    orientation,
                    (0, 0),
                    (0, 0),
                )?;
                LayoutContainer::add_view(&mut self.layout, view);
            }
        }
        Ok(())
    }

    /// Set the window level for a specific view.
    pub fn set_window_level(&mut self, index: usize, window_level: f32) -> Result<(), String> {
        if let Some(view) = self.layout.views_mut().get_mut(index) {
            if let Some(mpr_view) = view.as_any_mut().downcast_mut::<MprView>() {
                mpr_view
                    .set_window_level(window_level)
                    .map_err(|e| e.to_string())?;
                Ok(())
            } else {
                Err(format!("View {} is not an MPR view", index))
            }
        } else {
            Err(format!("View index {} out of bounds", index))
        }
    }

    /// Set the window width for a specific view.
    pub fn set_window_width(&mut self, index: usize, window_width: f32) -> Result<(), String> {
        if let Some(view) = self.layout.views_mut().get_mut(index) {
            if let Some(mpr_view) = view.as_any_mut().downcast_mut::<MprView>() {
                mpr_view
                    .set_window_width(window_width)
                    .map_err(|e| e.to_string())?;
                Ok(())
            } else {
                Err(format!("View {} is not an MPR view", index))
            }
        } else {
            Err(format!("View index {} out of bounds", index))
        }
    }

    /// Set the slice position (Z-coordinate) for a specific view.
    pub fn set_slice_mm(&mut self, index: usize, z: f32) -> Result<(), String> {
        if let Some(view) = self.layout.views_mut().get_mut(index) {
            if let Some(mpr_view) = view.as_any_mut().downcast_mut::<MprView>() {
                mpr_view.set_slice_mm(z).map_err(|e| e.to_string())?;
                Ok(())
            } else {
                Err(format!("View {} is not an MPR view", index))
            }
        } else {
            Err(format!("View index {} out of bounds", index))
        }
    }

    /// Set the scale (zoom level) for a specific view.
    pub fn set_scale(&mut self, index: usize, scale: f32) -> Result<(), String> {
        if let Some(view) = self.layout.views_mut().get_mut(index) {
            let mut handled = false;
            if let Some(mpr_view) = view.as_any_mut().downcast_mut::<MprView>() {
                mpr_view.set_scale(scale).map_err(|e| e.to_string())?;
                handled = true;
            }
            if let Some(mip_view) = view.as_any_mut().downcast_mut::<MipView>() {
                mip_view.set_scale(scale); // MipView set_scale doesn't return Result currently?
                handled = true;
            }
            if handled {
                Ok(())
            } else {
                Err(format!("View {} does not support scaling", index))
            }
        } else {
            Err(format!("View index {} out of bounds", index))
        }
    }

    /// Set the translation in screen coordinates.
    pub fn set_translate_in_screen_coord(
        &mut self,
        index: usize,
        translate: [f32; 3],
    ) -> Result<(), String> {
        if let Some(view) = self.layout.views_mut().get_mut(index) {
            if let Some(mpr_view) = view.as_any_mut().downcast_mut::<MprView>() {
                mpr_view
                    .set_translate_in_screen_coord(translate)
                    .map_err(|e| e.to_string())?;
                Ok(())
            } else {
                Err(format!("View {} is not an MPR view", index))
            }
        } else {
            Err(format!("View index {} out of bounds", index))
        }
    }

    /// Set the pan (X, Y) for a specific view.
    pub fn set_pan(&mut self, index: usize, x: f32, y: f32) -> Result<(), String> {
        if let Some(view) = self.layout.views_mut().get_mut(index) {
            let mut handled = false;
            if let Some(mpr_view) = view.as_any_mut().downcast_mut::<MprView>() {
                mpr_view.set_pan(x, y).map_err(|e| e.to_string())?;
                handled = true;
            }
            if let Some(mip_view) = view.as_any_mut().downcast_mut::<MipView>() {
                mip_view.set_pan(x, y);
                handled = true;
            }
            if handled {
                Ok(())
            } else {
                Err(format!("View {} does not support panning", index))
            }
        } else {
            Err(format!("View index {} out of bounds", index))
        }
    }

    /// Set the pan in millimeters.
    pub fn set_pan_mm(&mut self, index: usize, x_mm: f32, y_mm: f32) -> Result<(), String> {
        if let Some(view) = self.layout.views_mut().get_mut(index) {
            if let Some(mpr_view) = view.as_any_mut().downcast_mut::<MprView>() {
                mpr_view.set_pan_mm(x_mm, y_mm).map_err(|e| e.to_string())?;
                Ok(())
            } else {
                Err(format!("View {} is not an MPR view", index))
            }
        } else {
            Err(format!("View index {} out of bounds", index))
        }
    }

    /// Set the center point in millimeters.
    pub fn set_center_at_point_in_mm(
        &mut self,
        index: usize,
        point: [f32; 3],
    ) -> Result<[f32; 3], String> {
        if let Some(view) = self.layout.views_mut().get_mut(index) {
            if let Some(mpr_view) = view.as_any_mut().downcast_mut::<MprView>() {
                mpr_view
                    .set_center_at_point_in_mm(point)
                    .map_err(|e| e.to_string())
            } else {
                Err(format!("View {} is not an MPR view", index))
            }
        } else {
            Err(format!("View index {} out of bounds", index))
        }
    }

    /// Set the slab thickness for a MIP view.
    pub fn set_slab_thickness(&mut self, index: usize, thickness: f32) -> Result<(), String> {
        if let Some(view) = self.layout.views_mut().get_mut(index) {
            if let Some(mip_view) = view.as_any_mut().downcast_mut::<MipView>() {
                mip_view.set_slab_thickness(thickness);
                Ok(())
            } else {
                Err(format!("View {} is not a MIP view", index))
            }
        } else {
            Err(format!("View index {} out of bounds", index))
        }
    }

    /// Set the MIP mode (MIP vs MinIP).
    pub fn set_mip_mode(&mut self, index: usize, mode: u32) -> Result<(), String> {
        if let Some(view) = self.layout.views_mut().get_mut(index) {
            if let Some(mip_view) = view.as_any_mut().downcast_mut::<MipView>() {
                mip_view.set_mip_mode(mode);
                Ok(())
            } else {
                Err(format!("View {} is not a MIP view", index))
            }
        } else {
            Err(format!("View index {} out of bounds", index))
        }
    }

    pub fn set_mip_rotation_angle_degrees(
        &mut self,
        index: usize,
        roll_deg: f32,
        yaw_deg: f32,
        pitch_deg: f32,
    ) -> Result<(), String> {
        if let Some(view) = self.layout.views_mut().get_mut(index) {
            if let Some(mip_view) = view.as_any_mut().downcast_mut::<MipView>() {
                mip_view.set_rotation_degrees(roll_deg, yaw_deg, pitch_deg);
                Ok(())
            } else {
                Err(format!("View {} is not a MIP view", index))
            }
        } else {
            Err(format!("View index {} out of bounds", index))
        }
    }
}
