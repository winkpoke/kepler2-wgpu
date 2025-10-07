//! View layout module
//!
//! Provides abstractions and implementations to arrange `View` instances in a parent area.
//! The `LayoutStrategy` trait defines how each view is positioned and sized, and `Layout<T>`
//! orchestrates the collection of views, delegating to the chosen strategy for placement.
//!
//! Invariants and behavior:
//! - Strategies compute positions in pixel coordinates, top-left origin.
//! - Sizes are non-negative; callers should avoid zero or negative results.
//! - The container triggers `move_to` and `resize` on views when added or when the parent dims change.
//!
//! TODO: Add property-based tests to ensure positions and sizes are within bounds and do not overflow.
//! TODO: Consider explicit visibility semantics in `View` to avoid offscreen hiding.
#![allow(dead_code)]


use super::{Renderable, View};

/// A strategy that computes per-view position and size within a parent dimension.
///
/// Responsibilities:
/// - Given `index`, `_total_views`, and `parent_dim` (width, height), return the origin `(x, y)`
///   and size `(width, height)` for the view.
///
/// Notes:
/// - Implementations should avoid panics and handle edge cases like small parent sizes.
/// - `_total_views` may be unused by some strategies, but can help for dynamic spacing.
pub trait LayoutStrategy {
    // fn layout(&self, views: &mut Vec<Box<dyn View>>, dim: (u32, u32));
    /// Calculate position and size for a single view.
    ///
    /// Returns `((x, y), (width, height))` describing the view rectangle.
    fn calculate_position_and_size(
        &self,
        index: u32,
        _total_views: u32,
        parent_dim: (u32, u32),
    ) -> ((i32, i32), (u32, u32));
}

/// Grid layout strategy.
///
/// Divides the parent area into a uniform grid of `rows × cols` cells with `spacing` in pixels
/// applied between adjacent cells on both axes. Views are assigned in row-major order.
pub struct GridLayout {
    /// Number of rows in the grid.
    pub rows: u32,
    /// Number of columns in the grid.
    pub cols: u32,
    /// Spacing in pixels between cells (horizontal and vertical).
    pub spacing: u32,
}

impl LayoutStrategy for GridLayout {
    /// Compute the origin and size for the cell corresponding to `index`.
    ///
    /// Notes:
    /// - Current implementation assumes `rows > 0` and `cols > 0`.
    /// - Large `spacing` relative to `parent_dim` can lead to underflow; consider saturating math.
    fn calculate_position_and_size(
        &self,
        index: u32,
        total_views: u32,
        parent_dim: (u32, u32),
    ) -> ((i32, i32), (u32, u32)) {
        // TODO: Use `saturating_sub` and guard zero divisions to prevent underflow/overflow.
        let cell_width = (parent_dim.0 - (self.cols - 1) * self.spacing) / self.cols;
        let cell_height = (parent_dim.1 - (self.rows - 1) * self.spacing) / self.rows;
        let row = index / self.cols;
        let col = index % self.cols;
        let x = col as i32 * (cell_width + self.spacing) as i32;
        let y = row as i32 * (cell_height + self.spacing) as i32;
        ((x, y), (cell_width, cell_height))
    }
}

/// Single-cell layout: displays only the first view and hides others.
///
/// The first view fills the parent dimensions; subsequent views are moved offscreen and
/// given a minimal size to hide them.
///
/// Limitations:
/// - Offscreen hiding can be brittle; consider explicit visibility or zero-size rects.
pub struct OneCellLayout {
    /// Unused in current logic; kept for symmetry and potential future use.
    pub rows: u32,
    /// Unused in current logic; kept for symmetry and potential future use.
    pub cols: u32,
    /// Unused in current logic; kept for symmetry and potential future use.
    pub spacing: u32,
}

impl LayoutStrategy for OneCellLayout {
    /// Compute position and size where only `index == 0` occupies the parent; others are hidden.
    fn calculate_position_and_size(
        &self,
        index: u32,
        total_views: u32,
        parent_dim: (u32, u32),
    ) -> ((i32, i32), (u32, u32)) {
        if total_views <= 1 || index == 0 {
            ((0, 0),  parent_dim)
        } else {
            // TODO: Replace offscreen hiding with explicit visibility or zero-size rectangles.
            // When there are multiple views, other views are not displayed.
            ((-1000, -1000), (1, 1))
        }
    }
}
/// A generic layout container parameterized by a `LayoutStrategy`.
///
/// Manages a collection of `View` instances and delegates placement to `strategy` on view addition
/// and when the parent dimensions change. Also forwards update and render calls to each child view.
pub struct Layout <T: LayoutStrategy> {
    /// Parent dimensions `(width, height)` in pixels.
    dim: (u32, u32),
    /// The layout strategy used to compute positions and sizes.
    pub(crate) strategy: T,
    /// Collection of views arranged according to the strategy.
    pub(crate) views: Vec<Box<dyn View>>, // A collection of views
}

impl<T: LayoutStrategy> Layout<T> {
    /// Create a new layout with the given parent dimensions and strategy.
    pub fn new(dim: (u32, u32), strategy: T) -> Self {
        Self {
            dim,
            strategy,
            views: Vec::new(),
        }
    }

    /// Add a view to the container and immediately place and size it via the strategy.
    ///
    /// Notes:
    /// - Index used for placement is derived from the current number of views.
    /// - Logs at `info` level; consider `debug` to reduce noise in hot paths.
    /// - For grid-like strategies, consider caching cell size and recomputing only on resize.
    pub fn add_view(&mut self, mut view: Box<dyn View>) {
        let idx = self.views.len() as u32;
        let total_views = (self.views.len() + 1) as u32;
        let (pos, size) = self.strategy.calculate_position_and_size(idx, total_views, self.dim);
        log::info!("Adding view at position: {:?} with size: {:?}", pos, size);
        view.move_to(pos);
        view.resize(size);
        self.views.push(view);
    }

    /// Get a reference to a view by index, or `None` if out of bounds.
    ///
    /// TODO: Return `Option<&dyn View>` (and a mutable variant) to avoid exposing `Box`.
    pub fn get_view_by_index(&self, index: usize) -> Option<&Box<dyn View>> {
        // check if the index is within bounds
        if index >= self.views.len() {
            return None;
        }
        self.views.get(index)
    }

    /// Remove all views from the container.
    pub fn remove_all(&mut self) {
        self.views.clear();
    }

    /// Resize the parent dimensions and recompute each view's position and size.
    ///
    /// TODO: Cache per-cell dimensions for strategies like `GridLayout` to minimize repeated math.
    /// TODO: Consider center alignment and remainder distribution for better visual balance.
    pub fn resize(&mut self, dim: (u32, u32)) {
        self.dim = dim;
        let total_views = self.views.len() as u32;
        for (i, view) in self.views.iter_mut().enumerate() {
            let (pos, size) = self.strategy.calculate_position_and_size(i as u32, total_views, self.dim);
            view.move_to(pos);
            view.resize(size);
        }
    }
}

impl<T: LayoutStrategy> Renderable for Layout<T> {
    /// Update all child views. Typically called per-frame before rendering.
    fn update(&mut self, queue: &wgpu::Queue) {
        for v in &mut self.views {
            v.update(queue);
        }
    }

    /// Render all child views sequentially.
    ///
    /// Error handling: Propagates `wgpu::SurfaceError` from child views and stops on first failure.
    fn render(&mut self, render_pass: &mut wgpu::RenderPass) -> Result<(), wgpu::SurfaceError> {
        for v in &mut self.views {
            v.render(render_pass)?;
        }
        Ok(())
    }
}
