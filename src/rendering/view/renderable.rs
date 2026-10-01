pub trait Renderable {
    fn update(&mut self, queue: &wgpu::Queue);
    fn render(
        &mut self,
        render_pass: &mut wgpu::RenderPass,
    ) -> crate::core::error::KeplerResult<()>;
}
