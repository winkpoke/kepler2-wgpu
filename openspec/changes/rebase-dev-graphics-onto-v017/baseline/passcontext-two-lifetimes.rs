#![allow(dead_code)]
pub struct PassContext<'pass, 'encoder> { pub pass: &'pass mut wgpu::RenderPass<'encoder> }
fn execute<F>(encoder: &mut wgpu::CommandEncoder, descriptor: &wgpu::RenderPassDescriptor<'_>, mut callback: F)
where F: FnMut(PassContext<'_, '_>) {
    let mut pass = encoder.begin_render_pass(descriptor);
    callback(PassContext { pass: &mut pass });
}
