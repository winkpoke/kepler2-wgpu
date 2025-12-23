//! AppView: centralizes layout and view factory ownership
//!
//! Minimal AppView that owns DynamicLayout and DefaultViewFactory.
//! State will hold AppView and forward calls, keeping existing render loop intact.

use std::sync::Arc;
use crate::rendering::{GridLayout, LayoutContainer, OneCellLayout};
use crate::rendering::view::{DynamicLayout, DefaultViewFactory, View, Orientation, ALL_ORIENTATIONS};
use crate::rendering::view::ViewFactory;
use crate::rendering::view::render_content::RenderContent;
use crate::rendering::view::mesh::mesh::Mesh;
use crate::CTVolume;

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
    pub fn replace_view_at(&mut self, index: usize, new_view: Box<dyn View>) -> Option<Box<dyn View>> {
        LayoutContainer::replace_view_at(&mut self.layout, index, new_view)
    }

    /// Switch layout strategy to a single-cell layout (OneCellLayout).
    ///
    /// Function-level comment: Centralizes strategy changes through AppView for active-view workflows.
    pub fn set_one_cell_layout(&mut self) {
        self.layout.set_strategy(Box::new(OneCellLayout { rows: 1, cols: 1, spacing: 0 }));
    }

    /// Switch layout strategy to a grid layout.
    ///
    /// Function-level comment: Exposes grid layout configuration via AppView.
    pub fn set_grid_layout(&mut self, rows: u32, cols: u32, spacing: u32) {
        self.layout.set_strategy(Box::new(GridLayout { rows, cols, spacing }));
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
        size: (u32, u32)
    ) -> Result<(), Box<dyn std::error::Error>> {
        let view = self.view_factory.create_mpr_view(vol, orientation, pos, size)?;
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
        size: (u32, u32)
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
        size: (u32, u32)
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
        size: (u32, u32)
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
        size: (u32, u32)
    ) -> Result<(), Box<dyn std::error::Error>> {
        let view = self.view_factory.create_mip_view_with_content(render_content, pos, size)?;
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
            let mip_view = self.view_factory.create_mip_view_with_content(texture.clone(), (0, 0), (0, 0))?;
            LayoutContainer::replace_view_at(&mut self.layout, mip.unwrap(), mip_view);
        }
        
        if mesh_index.is_some() {
            let mesh_view = self.view_factory.create_mesh_view_with_content(texture.clone(),&cached_mesh.unwrap(), (0, 0), (0, 0))?;
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
        orientation_index: usize
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.set_one_cell_layout();
        self.remove_all();

        match mode {
            0 => { // MPR
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
            1 => { // MIP
                let mip_view = self.view_factory.create_mip_view_with_content(texture.clone(), (0, 0), (0, 0))?;
                LayoutContainer::add_view(&mut self.layout, mip_view);
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
}