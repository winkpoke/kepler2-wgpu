use super::{Renderable, View};

pub trait LayoutStrategy {
    // fn layout(&self, views: &mut Vec<Box<dyn View>>, dim: (u32, u32));
    fn calculate_position_and_size(
        &self,
        index: u32,
        total_views: u32,
        parent_dim: (u32, u32),
    ) -> ((i32, i32), (u32, u32));
}

pub struct GridLayout {
    pub rows: u32,
    pub cols: u32,
    pub spacing: u32,
}

impl LayoutStrategy for GridLayout {
    fn calculate_position_and_size(
        &self,
        index: u32,
        total_views: u32,
        parent_dim: (u32, u32),
    ) -> ((i32, i32), (u32, u32)) {
        let cell_width = (parent_dim.0 - (self.cols - 1) * self.spacing) / self.cols;
        let cell_height = (parent_dim.1 - (self.rows - 1) * self.spacing) / self.rows;
        let row = index / self.cols;
        let col = index % self.cols;
        let x = col as i32 * (cell_width + self.spacing) as i32;
        let y = row as i32 * (cell_height + self.spacing) as i32;
        ((x, y), (cell_width, cell_height))
    }
}

pub struct Layout <T: LayoutStrategy> {
    dim: (u32, u32),
    strategy: T,
    pub(crate) views: Vec<Box<dyn Renderable>>, // A collection of views
}

impl<T: LayoutStrategy> Layout<T> {
    pub fn new(dim: (u32, u32), strategy: T) -> Self {
        Self {
            dim,
            strategy,
            views: Vec::new(),
        }
    }

    pub fn add_view(&mut self, mut view: Box<dyn View>) {
        let idx = self.views.len() as u32;
        let total_views = (self.views.len() + 1) as u32;
        let (pos, size) = self.strategy.calculate_position_and_size(idx, total_views, self.dim);
        view.move_to(pos);
        view.resize(size);
        self.views.push(view);
    }
}

impl<T: LayoutStrategy> Renderable for Layout<T> {
    fn update(&mut self, queue: &wgpu::Queue) {
        for v in &mut self.views {
            v.update(queue);
        }
    }

    fn render(&mut self, render_pass: &mut wgpu::RenderPass) -> Result<(), wgpu::SurfaceError> {
        for v in &mut self.views {
            v.render(render_pass)?;
        }
        Ok(())
    }
}
