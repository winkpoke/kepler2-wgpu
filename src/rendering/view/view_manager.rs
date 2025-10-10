//! ViewManager module
//!
//! Provides centralized management for view transitions and state preservation.
//! The `ViewManager` handles saving and restoring view states during mode switches,
//! ensuring smooth transitions between different view types (MPR ↔ Mesh).
//!
//! Key responsibilities:
//! - State preservation during view transitions
//! - Centralized view creation through factory pattern
//! - Error handling and recovery for view operations
//! - Logging and debugging support for view management

use std::collections::HashMap;
use super::{View, ViewState, StatefulView, ViewFactory};

/// Centralized manager for view transitions and state preservation.
///
/// The ViewManager maintains a registry of view states indexed by position,
/// allowing seamless transitions between different view types while preserving
/// user configurations like zoom, pan, window/level settings, etc.
pub struct ViewManager {
    /// Registry of saved view states indexed by view position/identifier
    saved_states: HashMap<String, ViewState>,
    /// Factory for creating new view instances
    factory: Box<dyn ViewFactory>,
}

impl ViewManager {
    /// Create a new ViewManager with the specified factory.
    ///
    /// # Arguments
    /// * `factory` - Factory implementation for creating view instances
    pub fn new(factory: Box<dyn ViewFactory>) -> Self {
        log::info!("Initializing ViewManager with factory");
        Self {
            saved_states: HashMap::new(),
            factory,
        }
    }

    /// Save the state of a view at the specified position.
    ///
    /// This method extracts the current state from a StatefulView and stores it
    /// in the internal registry for later restoration.
    ///
    /// # Arguments
    /// * `position` - Unique identifier for the view position (e.g., "view_0", "main_view")
    /// * `view` - The view whose state should be saved
    ///
    /// # Returns
    /// * `Ok(())` if the state was successfully saved
    /// * `Err(String)` if the view doesn't support state management or saving failed
    pub fn save_view_state(&mut self, position: &str, view: &dyn View) -> Result<(), String> {
        // Try to downcast to StatefulView using the concrete type
        // Since we know GenericMPRView implements StatefulView, we can try that first
        if let Some(mpr_view) = view.as_any().downcast_ref::<super::GenericMPRView>() {
            if let Some(state) = mpr_view.save_state() {
                if state.is_valid() {
                    log::info!("Saving view state for position: {}", position);
                    log::debug!("Saved state: window_level={}, window_width={}, slice_mm={}, scale={}, translate={:?}", 
                        state.window_level, state.window_width, state.slice_mm, state.scale, state.translate);
                    
                    self.saved_states.insert(position.to_string(), state);
                    Ok(())
                } else {
                    let error = format!("Invalid view state for position: {}", position);
                    log::warn!("{}", error);
                    Err(error)
                }
            } else {
                let error = format!("Failed to save state for view at position: {}", position);
                log::warn!("{}", error);
                Err(error)
            }
        } else {
            let error = format!("View at position {} does not support state management", position);
            log::debug!("{}", error);
            Err(error)
        }
    }

    /// Restore the state of a view at the specified position.
    ///
    /// This method retrieves a previously saved state and applies it to the target view.
    ///
    /// # Arguments
    /// * `position` - Unique identifier for the view position
    /// * `view` - The view to which the state should be restored
    ///
    /// # Returns
    /// * `Ok(())` if the state was successfully restored
    /// * `Err(String)` if no saved state exists or restoration failed
    pub fn restore_view_state(&self, position: &str, view: &mut dyn View) -> Result<(), String> {
        if let Some(saved_state) = self.saved_states.get(position) {
            // Try to downcast to StatefulView using the concrete type
            if let Some(mpr_view) = view.as_any_mut().downcast_mut::<super::GenericMPRView>() {
                log::info!("Restoring view state for position: {}", position);
                log::debug!("Restoring state: window_level={}, window_width={}, slice_mm={}, scale={}, translate={:?}", 
                    saved_state.window_level, saved_state.window_width, saved_state.slice_mm, saved_state.scale, saved_state.translate);
                
                if mpr_view.restore_state(saved_state) {
                    Ok(())
                } else {
                    let error = format!("Failed to restore state for view at position: {}", position);
                    log::warn!("{}", error);
                    Err(error)
                }
            } else {
                let error = format!("View at position {} does not support state management", position);
                log::warn!("{}", error);
                Err(error)
            }
        } else {
            let error = format!("No saved state found for position: {}", position);
            log::debug!("{}", error);
            Err(error)
        }
    }

    /// Create a new mesh view using the configured factory.
    ///
    /// # Arguments
    /// * `manager` - Pipeline manager for resource creation
    /// * `pos` - Position (x, y) for the view
    /// * `size` - Size (width, height) for the view
    ///
    /// # Returns
    /// * `Ok(Box<dyn View>)` - Successfully created mesh view
    /// * `Err(String)` - Factory error or creation failure
    pub fn create_mesh_view(
        &self,
        manager: &mut crate::rendering::core::pipeline::PipelineManager,
        pos: (i32, i32),
        size: (u32, u32),
    ) -> Result<Box<dyn View>, String> {
        log::info!("Creating mesh view through ViewManager at pos: {:?}, size: {:?}", pos, size);
        self.factory.create_mesh_view(manager, pos, size)
            .map_err(|e| {
                log::error!("Failed to create mesh view: {}", e);
                format!("{}", e)
            })
    }

    /// Create a new MPR view using the configured factory.
    ///
    /// # Arguments
    /// * `manager` - Pipeline manager for resource creation
    /// * `vol` - CT volume data for MPR rendering
    /// * `orientation` - MPR orientation (Axial, Coronal, Sagittal)
    /// * `pos` - Position (x, y) for the view
    /// * `size` - Size (width, height) for the view
    ///
    /// # Returns
    /// * `Ok(Box<dyn View>)` - Successfully created MPR view
    /// * `Err(String)` - Factory error or creation failure
    pub fn create_mpr_view(
        &self,
        manager: &mut crate::rendering::core::pipeline::PipelineManager,
        vol: &crate::data::ct_volume::CTVolume,
        orientation: super::Orientation,
        pos: (i32, i32),
        size: (u32, u32),
    ) -> Result<Box<dyn View>, String> {
        log::info!("Creating MPR view through ViewManager with orientation: {:?} at pos: {:?}, size: {:?}", 
                   orientation, pos, size);
        self.factory.create_mpr_view(manager, vol, orientation, pos, size)
            .map_err(|e| {
                log::error!("Failed to create MPR view: {}", e);
                format!("{}", e)
            })
    }

    /// Clear all saved states.
    ///
    /// This method removes all stored view states, typically used during
    /// application reset or when switching to a completely new dataset.
    pub fn clear_states(&mut self) {
        log::info!("Clearing all saved view states (count: {})", self.saved_states.len());
        self.saved_states.clear();
    }

    /// Get the number of saved states.
    pub fn saved_state_count(&self) -> usize {
        self.saved_states.len()
    }

    /// Check if a saved state exists for the specified position.
    pub fn has_saved_state(&self, position: &str) -> bool {
        self.saved_states.contains_key(position)
    }

    /// Remove a specific saved state.
    ///
    /// # Arguments
    /// * `position` - Position identifier for the state to remove
    ///
    /// # Returns
    /// * `true` if the state was removed
    /// * `false` if no state existed for the position
    pub fn remove_saved_state(&mut self, position: &str) -> bool {
        if self.saved_states.remove(position).is_some() {
            log::debug!("Removed saved state for position: {}", position);
            true
        } else {
            log::debug!("No saved state found to remove for position: {}", position);
            false
        }
    }
}

impl std::fmt::Debug for ViewManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ViewManager")
            .field("saved_states_count", &self.saved_states.len())
            .field("saved_positions", &self.saved_states.keys().collect::<Vec<_>>())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rendering::view::Orientation;

    // Mock factory for testing
    struct MockViewFactory;
    
    impl ViewFactory for MockViewFactory {
        fn create_mesh_view(
            &self,
            _manager: &mut crate::rendering::core::pipeline::PipelineManager,
            _pos: (i32, i32),
            _size: (u32, u32),
        ) -> Result<Box<dyn View>, Box<dyn std::error::Error>> {
            Err("Mock factory - not implemented".into())
        }

        fn create_mpr_view(
            &self,
            _manager: &mut crate::rendering::core::pipeline::PipelineManager,
            _vol: &crate::data::ct_volume::CTVolume,
            _orientation: Orientation,
            _pos: (i32, i32),
            _size: (u32, u32),
        ) -> Result<Box<dyn View>, Box<dyn std::error::Error>> {
            Err("Mock factory - not implemented".into())
        }
    }

    #[test]
    fn test_view_manager_creation() {
        let factory = Box::new(MockViewFactory);
        let manager = ViewManager::new(factory);
        
        assert_eq!(manager.saved_state_count(), 0);
        assert!(!manager.has_saved_state("test_position"));
    }

    #[test]
    fn test_state_management() {
        let factory = Box::new(MockViewFactory);
        let mut manager = ViewManager::new(factory);
        
        // Test state operations
        assert!(!manager.has_saved_state("test"));
        assert!(!manager.remove_saved_state("test"));
        
        manager.clear_states();
        assert_eq!(manager.saved_state_count(), 0);
    }
}