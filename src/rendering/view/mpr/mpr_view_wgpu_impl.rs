#![allow(dead_code)]

use super::mpr_render_context::MprRenderContext;
use crate::rendering::view::NeedleUniform;
use crate::rendering::view::render_content::RenderContent;
use crate::rendering::view::LABEL_COLORS;
use std::sync::Arc;

/// Uniform data structures for MPR rendering
#[repr(C)]
#[derive(Copy, Clone, Default, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct UniformsVert {
    pub _padding: [f32; 4],
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct UniformsFrag {
    pub window_width: f32,
    pub window_level: f32,
    pub slice: f32,
    pub is_packed_rg8: f32,
    pub bias: f32,
    pub is_dual_mode: f32,
    pub slice2: f32,
    pub aliasing: u32,  // Change from bool to u32
    pub mat: [f32; 16],
    pub needle_count: u32,
    pub needle_enabled: f32,
    pub seg_enabled: f32,
    pub _pad0: f32,
    pub needles: [NeedleUniform; 32],
    pub label_colors: [[f32; 4]; 8],
    pub label_visibility: [[f32; 4]; 8],
}

impl Default for UniformsFrag {
    fn default() -> Self {
        Self {
            window_width: 0.0,
            window_level: 0.0,
            slice: 0.0,
            is_packed_rg8: 0.0,
            bias: 0.0,
            is_dual_mode: 0.0,
            slice2: 0.0,
            aliasing: 0,
            mat: [0.0; 16],
            needle_count: 0,
            needle_enabled: 0.0,
            seg_enabled: 0.0,
            _pad0: 0.0,
            needles: [NeedleUniform::default(); 32],
            label_colors: LABEL_COLORS,
            label_visibility: [[0.0; 4]; 8],
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Default, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Uniforms {
    pub vert: UniformsVert,
    pub frag: UniformsFrag,
}

/// Per-view GPU implementation for MPR rendering
/// Contains view-specific GPU resources like bind groups and uniform buffers
/// References shared context and content for resource efficiency
pub struct MprViewWgpuImpl {
    /// Reference to shared global GPU state
    pub render_context: Arc<MprRenderContext>,

    /// Reference to shared texture content
    pub render_content: Arc<RenderContent>,

    /// Active segmentation texture
    pub seg_content: Arc<RenderContent>,

    /// View-specific texture bind group (volume + segmentation)
    pub texture_bind_group: wgpu::BindGroup,

    /// View-specific vertex uniform buffer
    pub uniform_vert_buffer: wgpu::Buffer,

    /// View-specific vertex uniform bind group
    pub uniform_vert_bind_group: wgpu::BindGroup,

    /// View-specific fragment uniform buffer
    pub uniform_frag_buffer: wgpu::Buffer,

    /// View-specific fragment uniform bind group
    pub uniform_frag_bind_group: wgpu::BindGroup,

    /// Current uniform values for this view
    pub uniforms: Uniforms,
}

impl MprViewWgpuImpl {
    /// Create a new MprViewWgpuImpl with view-specific GPU resources
    ///
    /// # Arguments
    /// * `render_context` - Shared global GPU state
    /// * `device` - WGPU device for creating GPU resources
    /// * `render_content` - Shared texture content
    /// * `transform_matrix` - 4x4 matrix for view transforms
    ///
    /// # Returns
    /// A new MprViewWgpuImpl with initialized per-view resources
    pub fn new(
        render_context: Arc<MprRenderContext>,
        device: &wgpu::Device,
        render_content: Arc<RenderContent>,
        transform_matrix: glam::Mat4,
    ) -> Self {
        // Initialize uniform data
        let u_vert_data = UniformsVert {
            ..Default::default()
        };

        let decode_params = render_content.decode_parameters();
        let u_frag_data = UniformsFrag {
            window_width: 350.,
            window_level: 40.0,
            slice: 0.0,
            is_packed_rg8: decode_params.is_packed_flag as f32,
            bias: decode_params.bias,
            is_dual_mode: 0.0,
            slice2: 0.0,
            aliasing: 0,
            mat: transform_matrix.to_cols_array(),
            needle_count: 0,
            needle_enabled: 0.0,
            seg_enabled: 0.0,
            _pad0: 0.0,
            needles: [NeedleUniform::default(); 32],
            label_colors: LABEL_COLORS,
            label_visibility: [[0.0; 4]; 8],
        };

        log::info!(
            "MprViewWgpuImpl defaults => window_width: {:.1}, window_level: {:.1}, is_packed_rg8: {}",
            u_frag_data.window_width,
            u_frag_data.window_level,
            decode_params.is_packed_flag
        );

        let uniforms = Uniforms {
            vert: u_vert_data,
            frag: u_frag_data,
        };

        // Start with the shared default (1x1x1 zero label) seg content.
        let seg_content = Arc::clone(&render_context.default_seg_content);

        // Create view-specific texture bind group using shared layout.
        // Bindings 0/1 = volume texture + sampler (filterable).
        // Bindings 2/3 = segmentation texture + sampler (R8Uint,
        // non-filterable). The bind group must include all 4 entries
        // because the layout declares them.
        let texture_bind_group = Self::create_texture_bind_group(
            device,
            &render_context.texture_bind_group_layout,
            &render_content,
            &seg_content,
        );

        // Create view-specific vertex uniform buffer and bind group
        let (uniform_vert_buffer, uniform_vert_bind_group) = Self::create_vertex_uniform_bind_group(
            device,
            &render_context.vertex_bind_group_layout,
            &uniforms.vert,
        );

        // Create view-specific fragment uniform buffer and bind group
        let (uniform_frag_buffer, uniform_frag_bind_group) =
            Self::create_fragment_uniform_bind_group(
                device,
                &render_context.fragment_bind_group_layout,
                &uniforms.frag,
            );

        log::debug!("MprViewWgpuImpl created with view-specific GPU resources");

        Self {
            render_context,
            render_content,
            seg_content,
            texture_bind_group,
            uniform_vert_buffer,
            uniform_vert_bind_group,
            uniform_frag_buffer,
            uniform_frag_bind_group,
            uniforms,
        }
    }

    /// Helper that builds a texture bind group covering both the volume
    /// (bindings 0/1) and the segmentation texture (bindings 2/3).
    fn create_texture_bind_group(
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
        volume: &RenderContent,
        seg: &RenderContent,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&volume.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&volume.sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&seg.view),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Sampler(&seg.sampler),
                },
            ],
            label: Some("mpr_view_texture_bind_group"),
        })
    }

    /// Swap the segmentation texture and rebuild the texture bind group
    /// so the new R8Uint label volume is sampled by the fragment shader.
    /// Also toggles `seg_enabled` to 1.0 (use the overlay). 
    /// Pass `None` to revert to the default empty texture and disable the overlay.
    pub fn set_segmentation(
        &mut self,
        device: &wgpu::Device,
        seg: Option<Arc<RenderContent>>,
    ) {
        let enable = seg.is_some();
        self.seg_content = match seg {
            Some(c) => c,
            None => Arc::clone(&self.render_context.default_seg_content),
        };
        self.texture_bind_group = Self::create_texture_bind_group(
            device,
            &self.render_context.texture_bind_group_layout,
            &self.render_content,
            &self.seg_content,
        );
        self.uniforms.frag.seg_enabled = if enable { 1.0 } else { 0.0 };
        log::info!(
            "MprViewWgpuImpl: segmentation {} ({}x{}x{})",
            if self.uniforms.frag.seg_enabled > 0.5 { "enabled" } else { "disabled" },
            self.seg_content.texture.size().width,
            self.seg_content.texture.size().height,
            self.seg_content.texture.size().depth_or_array_layers,
        );
    }

    pub fn set_segmentation_visibility(
        &mut self, 
        queue: &wgpu::Queue, 
        mask: [f32; 8]
    ){
        for (i, &v) in mask.iter().enumerate() {
            if let Some(slot) = self.uniforms.frag.label_visibility.get_mut(i) {
                slot[0] = if v > 0.5 { 1.0 } else { 0.0 };
            }
        }
        self.update_uniforms_buffers(queue);
    }

    /// Set the uniform values for this view
    ///
    /// # Arguments
    /// * `new_uniforms` - New uniform values to set
    pub fn set_uniforms(&mut self, new_uniforms: Uniforms) {
        // Use the separate set methods for better modularity and consistency
        self.set_vertex_uniforms(new_uniforms.vert);
        self.set_fragment_uniforms(new_uniforms.frag);

        log::trace!("Set both vertex and fragment uniform values");
    }

    /// Set transformation matrix
    ///
    /// # Arguments
    /// * `matrix` - New transformation matrix
    pub fn set_matrix(&mut self, matrix: [f32; 16]) {
        self.uniforms.frag.mat = matrix;
    }

    /// Set slice position
    ///
    /// # Arguments
    /// * `slice` - New slice position
    pub fn set_slice(&mut self, slice: f32) {
        self.uniforms.frag.slice = slice;
    }

    /// Set second slice position
    pub fn set_slice2(&mut self, slice: f32) {
        self.uniforms.frag.slice2 = slice;
    }

    /// Set antialiasing flag
    pub fn set_aliasing(&mut self, aliasing: bool) {
        self.uniforms.frag.aliasing = if aliasing { 1 } else { 0 };
    }

    /// Set window level only
    ///
    /// # Arguments
    /// * `window_level` - New window level value
    pub fn set_window_level(&mut self, window_level: f32) {
        self.uniforms.frag.window_level = window_level;
    }

    /// Set window width only
    ///
    /// # Arguments
    /// * `window_width` - New window width value
    pub fn set_window_width(&mut self, window_width: f32) {
        self.uniforms.frag.window_width = window_width;
    }

    /// Set only the vertex uniform values
    ///
    /// # Arguments
    /// * `vertex_uniforms` - New vertex uniform values to set
    pub fn set_vertex_uniforms(&mut self, vertex_uniforms: UniformsVert) {
        self.uniforms.vert = vertex_uniforms;

        log::trace!("Set vertex uniform values");
    }

    /// Set only the fragment uniform values
    ///
    /// # Arguments
    /// * `fragment_uniforms` - New fragment uniform values to set
    pub fn set_fragment_uniforms(&mut self, fragment_uniforms: UniformsFrag) {
        self.uniforms.frag = fragment_uniforms;

        log::trace!("Set fragment uniform values with window_width: {:.1}, window_level: {:.1}, slice: {:.1}",
                   fragment_uniforms.window_width, fragment_uniforms.window_level, fragment_uniforms.slice);
    }

    /// Replace the entire needle array (up to 32 entries) and its enabled flag.
    /// The caller is responsible for keeping the slice short enough to fit.
    pub fn set_needles(&mut self, needles: &[NeedleUniform], enabled: bool) {
        let mut gpu_needles = [NeedleUniform::default(); 32];
        let n = needles.len().min(32);
        for (i, ndl) in needles.iter().take(n).enumerate() {
            gpu_needles[i] = *ndl;
        }
        self.uniforms.frag.needles = gpu_needles;
        self.uniforms.frag.needle_count = n as u32;
        self.uniforms.frag.needle_enabled = if enabled { 1.0 } else { 0.0 };
    }

    /// Toggle needle rendering on/off without touching the needle list.
    pub fn set_needles_enabled(&mut self, enabled: bool) {
        self.uniforms.frag.needle_enabled = if enabled { 1.0 } else { 0.0 };
    }

    /// Update vertex uniform buffer with current uniform values
    ///
    /// # Arguments
    /// * `queue` - WGPU queue for buffer updates
    pub fn update_vertex_uniforms_buffer(&self, queue: &wgpu::Queue) {
        queue.write_buffer(
            &self.uniform_vert_buffer,
            0,
            bytemuck::cast_slice(&[self.uniforms.vert]),
        );

        log::trace!("Updated vertex uniform buffer");
    }

    /// Update fragment uniform buffer with current uniform values
    ///
    /// # Arguments
    /// * `queue` - WGPU queue for buffer updates
    pub fn update_fragment_uniforms_buffer(&self, queue: &wgpu::Queue) {
        queue.write_buffer(
            &self.uniform_frag_buffer,
            0,
            bytemuck::cast_slice(&[self.uniforms.frag]),
        );

        log::trace!("Updated fragment uniform buffer with window_width: {:.1}, window_level: {:.1}, slice: {:.1}", 
                   self.uniforms.frag.window_width, self.uniforms.frag.window_level, self.uniforms.frag.slice);
    }

    /// Update both uniform buffers with current uniform values
    ///
    /// # Arguments
    /// * `queue` - WGPU queue for buffer updates
    pub fn update_uniforms_buffers(&self, queue: &wgpu::Queue) {
        self.update_vertex_uniforms_buffer(queue);
        self.update_fragment_uniforms_buffer(queue);

        log::trace!("Updated both vertex and fragment uniform buffers");
    }

    /// Helper function to create vertex uniform buffer and bind group
    fn create_vertex_uniform_bind_group(
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
        uniforms: &UniformsVert,
    ) -> (wgpu::Buffer, wgpu::BindGroup) {
        use wgpu::util::DeviceExt;

        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("MPR Vertex Uniform Buffer"),
            contents: bytemuck::cast_slice(&[*uniforms]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
            label: Some("MPR Vertex Uniform Bind Group"),
        });

        (buffer, bind_group)
    }

    /// Helper function to create fragment uniform buffer and bind group
    fn create_fragment_uniform_bind_group(
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
        uniforms: &UniformsFrag,
    ) -> (wgpu::Buffer, wgpu::BindGroup) {
        use wgpu::util::DeviceExt;

        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("MPR Fragment Uniform Buffer"),
            contents: bytemuck::cast_slice(&[*uniforms]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
            label: Some("MPR Fragment Uniform Bind Group"),
        });

        (buffer, bind_group)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Function-level comment: 验证片段统一缓冲结构体的尺寸与对齐（含专用填充字段）
    #[test]
    fn test_uniforms_frag_size_alignment() {
        let size = std::mem::size_of::<UniformsFrag>();
        // Layout:
        //   32 bytes header (7 f32 + 1 u32)
        // + 64 bytes mat
        // + 16 bytes needle header + _pad0 (1 u32 + 3 f32)
        // + 32 * 48 bytes NeedleUniform = 1536 bytes
        // + 8 * 16 bytes label_colors (vec4, 4th component is padding
        //   so the byte layout matches the WGSL `array<vec4<f32>, 8>`)
        // + 8 * 16 bytes label_visibility (vec4, only .x is read by the
        //   shader; vec4 stride matches WGSL `array<vec4<f32>, 8>`)
        // = 1904 bytes
        assert_eq!(size, 1904);
        let vert_size = std::mem::size_of::<UniformsVert>();
        assert_eq!(vert_size, 16);
        let uniforms_size = std::mem::size_of::<Uniforms>();
        assert_eq!(uniforms_size, vert_size + size);
    }
}
