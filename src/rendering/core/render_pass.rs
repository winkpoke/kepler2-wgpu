/// Render pass management system for separating 2D and 3D rendering
/// 
/// This module implements the architecture described in render-architecture-design.md
/// with separate passes for mesh (3D with depth) and slice (2D without depth) rendering.

use wgpu;
use std::collections::HashMap;
use crate::core::timing::{Instant, DurationExt};

use crate::rendering::mesh::mesh_texture_pool::MeshTexturePool;

type TexturePoolType = MeshTexturePool;

/// Unique identifier for render passes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PassId {
    /// 3D mesh rendering pass (offscreen with depth buffer)
    MeshPass,
    /// 2D slice rendering pass (onscreen without depth buffer)
    SlicePass,
    /// MIP (Maximum Intensity Projection) rendering pass (onscreen without depth buffer)
    MipPass,
}

/// Describes a render pass, including its name, output, and clearing behavior.
#[derive(Debug, Clone)]
pub struct PassDescriptor {
    /// A descriptive name for the render pass (e.g., "Mesh Pass").
    pub name: String,
    /// Whether this pass renders to an offscreen texture
    pub is_offscreen: bool,
    /// Color attachment format
    pub color_format: wgpu::TextureFormat,
    /// The color to clear the output texture with before rendering.
    pub clear_color: wgpu::Color,
    /// Whether to use a depth texture for this pass.
    pub uses_depth: bool,
    /// Whether to clear the depth buffer before rendering.
    pub clear_depth: bool,
}

impl PassDescriptor {
    /// Create a descriptor for the mesh pass (3D rendering with configurable depth)
    pub fn mesh_pass(surface_format: wgpu::TextureFormat, use_depth: bool) -> Self {
        Self {
            name: "MeshPass".to_string(),
            is_offscreen: false,  // Renders directly to surface
            color_format: surface_format,
            clear_color: wgpu::Color {
                r: 0.1,
                g: 0.1,
                b: 0.1,
                a: 1.0,
            },
            uses_depth: use_depth,
            clear_depth: use_depth,
        }
    }

    /// Create a descriptor for the slice pass (2D rendering without depth)
    pub fn slice_pass(surface_format: wgpu::TextureFormat) -> Self {
        Self {
            name: "SlicePass".to_string(),
            is_offscreen: false,
            color_format: surface_format,
            clear_color: wgpu::Color {
                r: 0.1,
                g: 0.1,
                b: 0.1,
                a: 1.0,
            },
            uses_depth: false,
            clear_depth: false,
        }
    }

    /// Create a descriptor for the MIP pass (Maximum Intensity Projection without depth)
    pub fn mip_pass(surface_format: wgpu::TextureFormat) -> Self {
        Self {
            name: "MipPass".to_string(),
            is_offscreen: false,
            color_format: surface_format,
            clear_color: wgpu::Color {
                r: 0.0,
                g: 0.0,
                b: 0.0,
                a: 1.0,
            },
            uses_depth: false,
            clear_depth: false,
        }
    }
}

/// Execution plan for a frame's render passes
#[derive(Debug)]
pub struct PassPlan {
    /// Ordered list of passes to execute
    pub passes: Vec<PassId>,
    /// Pass descriptors for each pass
    pub descriptors: HashMap<PassId, PassDescriptor>,
}

impl PassPlan {
    /// Create a new empty pass plan
    pub fn new() -> Self {
        Self {
            passes: Vec::new(),
            descriptors: HashMap::new(),
        }
    }

    /// Add a pass to the execution plan
    pub fn add_pass(&mut self, pass_id: PassId, descriptor: PassDescriptor) {
        if !self.passes.contains(&pass_id) {
            self.passes.push(pass_id);
        }
        self.descriptors.insert(pass_id, descriptor);
    }

    /// Get the descriptor for a pass
    pub fn get_descriptor(&self, pass_id: PassId) -> Option<&PassDescriptor> {
        self.descriptors.get(&pass_id)
    }

    /// Check if a pass is included in this plan
    pub fn has_pass(&self, pass_id: PassId) -> bool {
        self.passes.contains(&pass_id)
    }
}

/// Registry for managing render pass configurations
pub struct PassRegistry {
    /// Surface format for onscreen rendering
    surface_format: wgpu::TextureFormat,
}

impl PassRegistry {
    /// Create a new pass registry
    pub fn new(surface_format: wgpu::TextureFormat) -> Self {
        Self { surface_format }
    }

    /// Build a pass plan based on current rendering requirements
    /// 
    /// # Arguments
    /// * `mesh_enabled` - Whether 3D mesh rendering is enabled
    /// * `has_mesh_content` - Whether there is actual mesh content to render
    /// * `mip_enabled` - Whether MIP (Maximum Intensity Projection) rendering is enabled
    /// * `has_mip_content` - Whether there is actual MIP content to render
    /// 
    /// Render order follows the architecture design:
    /// 1. MeshPass (3D) renders first with Clear operation to establish base scene
    /// 2. MipPass (volume projection) renders second for 3D volume visualization
    /// 3. SlicePass (2D) renders third with Load operation to overlay on existing content
    pub fn build_pass_plan(&self, mesh_enabled: bool, has_mesh_content: bool, mip_enabled: bool, has_mip_content: bool) -> PassPlan {
        let mut plan = PassPlan::new();

        // Add mesh pass first for 3D rendering (base layer with Clear)
        if mesh_enabled && has_mesh_content {
            plan.add_pass(PassId::MeshPass, PassDescriptor::mesh_pass(self.surface_format, true));
        }

        // Add MIP pass second for volume projection rendering
        if mip_enabled && has_mip_content {
            plan.add_pass(PassId::MipPass, PassDescriptor::mip_pass(self.surface_format));
        }

        // Add slice pass third for 2D rendering (overlay with Load)
        plan.add_pass(PassId::SlicePass, PassDescriptor::slice_pass(self.surface_format));

        plan
    }

    /// Update surface format (e.g., when window is recreated)
    pub fn update_surface_format(&mut self, format: wgpu::TextureFormat) {
        self.surface_format = format;
    }
}

/// Render pass execution context
pub struct PassContext<'a> {
    /// The render pass being executed
    pub pass: &'a mut wgpu::RenderPass<'a>,
    /// The pass descriptor
    pub descriptor: &'a PassDescriptor,
    /// The pass ID
    pub pass_id: PassId,
}

impl<'a> PassContext<'a> {
    /// Create a new pass context
    pub fn new(pass: &'a mut wgpu::RenderPass<'a>, descriptor: &'a PassDescriptor, pass_id: PassId) -> Self {
        Self {
            pass,
            descriptor,
            pass_id,
        }
    }

    /// Check if this pass supports depth testing
    pub fn has_depth(&self) -> bool {
        self.descriptor.uses_depth
    }

    /// Check if this pass is offscreen
    pub fn is_offscreen(&self) -> bool {
        self.descriptor.is_offscreen
    }
}

/// Function-level comment: Error types for pass execution failures
#[derive(Debug, Clone)]
pub enum PassExecutionError {
    /// Pass descriptor missing
    MissingDescriptor(String),
    /// Texture creation or access failed
    TextureError(String),
    /// Render pass creation failed
    RenderPassCreationFailed(String),
    /// Rendering function failed
    RenderingFailed(String),
    /// Resource cleanup failed
    CleanupFailed(String),
}

impl std::fmt::Display for PassExecutionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PassExecutionError::MissingDescriptor(msg) => write!(f, "Missing pass descriptor: {}", msg),
            PassExecutionError::TextureError(msg) => write!(f, "Texture error: {}", msg),
            PassExecutionError::RenderPassCreationFailed(msg) => write!(f, "Render pass creation failed: {}", msg),
            PassExecutionError::RenderingFailed(msg) => write!(f, "Rendering failed: {}", msg),
            PassExecutionError::CleanupFailed(msg) => write!(f, "Cleanup failed: {}", msg),
        }
    }
}

impl std::error::Error for PassExecutionError {}

/// Function-level comment: Statistics for pass execution monitoring
#[derive(Debug, Default)]
pub struct PassExecutionStats {
    pub total_frames: u64,
    pub mesh_pass_failures: u64,
    pub slice_pass_failures: u64,
    pub texture_creation_failures: u64,
    pub last_error_time: Option<Instant>,
    pub consecutive_failures: u32,
}

/// Executor for managing and running render passes
pub struct PassExecutor {
    /// The pass registry
    registry: PassRegistry,
    /// Execution statistics and error tracking
    stats: PassExecutionStats,
    /// Whether mesh pass is temporarily disabled due to errors
    mesh_pass_disabled: bool,
    /// Last successful frame timestamp
    last_success_time: Instant,
}

impl PassExecutor {
    /// Create a new pass executor
    pub fn new(surface_format: wgpu::TextureFormat) -> Self {
        Self {
            registry: PassRegistry::new(surface_format),
            stats: PassExecutionStats::default(),
            mesh_pass_disabled: false,
            last_success_time: Instant::now(),
        }
    }

    /// Update the surface format
    pub fn update_surface_format(&mut self, format: wgpu::TextureFormat) {
        self.registry.update_surface_format(format);
    }
    
    /// Function-level comment: Get execution statistics for monitoring and debugging.
    pub fn get_stats(&self) -> &PassExecutionStats {
        &self.stats
    }
    
    /// Function-level comment: Check if the executor is in a healthy state.
    pub fn is_healthy(&self) -> bool {
        self.stats.consecutive_failures < 10 && 
        self.last_success_time.elapsed().as_secs_f64() < 60.0
    }
    
    /// Function-level comment: Reset error state and re-enable disabled passes.
    pub fn reset_error_state(&mut self) {
        self.stats.consecutive_failures = 0;
        self.mesh_pass_disabled = false;
        self.stats.last_error_time = None;
        log::info!("PassExecutor error state reset");
    }
    
    /// Function-level comment: Handle pass execution errors and determine recovery strategy.
    fn handle_pass_error(&mut self, pass_id: PassId, error: PassExecutionError) {
        self.stats.consecutive_failures += 1;
        self.stats.last_error_time = Some(Instant::now());
        
        match pass_id {
            PassId::MeshPass => {
                self.stats.mesh_pass_failures += 1;
                // Disable mesh pass if too many consecutive failures
                if self.stats.consecutive_failures >= 5 {
                    self.mesh_pass_disabled = true;
                    log::warn!("Mesh pass disabled due to repeated failures: {}", error);
                } else {
                    log::warn!("Mesh pass error (attempt {}): {}", self.stats.consecutive_failures, error);
                }
            }
            PassId::SlicePass => {
                self.stats.slice_pass_failures += 1;
                log::error!("Slice pass error (critical): {}", error);
                // Slice pass errors are critical since they affect 2D rendering
            }
            PassId::MipPass => {
                log::warn!("MIP pass error: {}", error);
                // MIP pass errors are non-critical, just log them
            }
        }
    }
    
    /// Function-level comment: Record successful pass execution.
    fn record_success(&mut self) {
        self.stats.total_frames += 1;
        self.last_success_time = Instant::now();
        
        // Reset consecutive failures on success
        if self.stats.consecutive_failures > 0 {
            log::info!("PassExecutor recovered after {} consecutive failures", self.stats.consecutive_failures);
            self.stats.consecutive_failures = 0;
        }
        
        // Re-enable mesh pass if it was disabled and we've had some successful frames
        if self.mesh_pass_disabled && self.stats.total_frames % 300 == 0 { // Try every 5 seconds at 60fps
            log::info!("Attempting to re-enable mesh pass after recovery period");
            self.mesh_pass_disabled = false;
        }
    }

    /// Execute render passes for a frame
    /// 
    /// # Arguments
    /// * `encoder` - Command encoder for recording render commands
    /// * `frame_view` - Surface texture view for final output
    /// * `texture_pool` - Texture pool for managing offscreen and depth textures
    /// * `device` - GPU device for resource creation
    /// * `surface_width` - Width of the surface for offscreen texture sizing
    /// * `surface_height` - Height of the surface for offscreen texture sizing
    /// * `mesh_enabled` - Whether mesh rendering is enabled
    /// * `has_mesh_content` - Whether there is mesh content to render
    /// * `mip_enabled` - Whether MIP rendering is enabled
    /// * `has_mip_content` - Whether there is MIP content to render
    /// * `render_fn` - Function to execute rendering for each pass
    pub fn execute_frame<F>(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        frame_view: &wgpu::TextureView,
        texture_pool: &mut TexturePoolType,
        device: &wgpu::Device,
        surface_width: u32,
        surface_height: u32,
        mesh_enabled: bool,
        has_mesh_content: bool,
        mip_enabled: bool,
        has_mip_content: bool,
        mut render_fn: F,
    ) -> Result<(), Box<dyn std::error::Error>>
    where
        F: FnMut(PassContext) -> Result<(), Box<dyn std::error::Error>>,
    {
        let frame_start_time = Instant::now();
        log::trace!("[FRAME_EXEC] Starting frame execution - Surface: {}x{}, Mesh enabled: {}, Has mesh content: {}, MIP enabled: {}, Has MIP content: {}", 
                    surface_width, surface_height, mesh_enabled, has_mesh_content, mip_enabled, has_mip_content);
        
        // Build the pass plan for this frame
        let effective_mesh_enabled = mesh_enabled && !self.mesh_pass_disabled;
        let plan = self.registry.build_pass_plan(effective_mesh_enabled, has_mesh_content, mip_enabled, has_mip_content);
        let mut frame_success = true;
        
        log::trace!("[FRAME_EXEC] Pass plan: {} passes scheduled, Effective mesh enabled: {}", 
                    plan.passes.len(), effective_mesh_enabled);
        if self.mesh_pass_disabled {
            log::warn!("[FRAME_EXEC] Mesh pass is currently DISABLED due to previous errors");
        }

        // Execute each pass in order
        for &pass_id in &plan.passes {
            let descriptor = plan.get_descriptor(pass_id)
                .ok_or("Missing pass descriptor")?;

            let pass_name = match pass_id {
                PassId::MeshPass => "MESH",
                PassId::SlicePass => "SLICE",
                PassId::MipPass => "MIP",
            };
            log::trace!("[FRAME_EXEC] Executing {} pass: '{}'", pass_name, descriptor.name);

            let pass_result = match pass_id {
                PassId::MeshPass => {
                    // Skip mesh pass if disabled due to errors
                    if self.mesh_pass_disabled {
                        log::trace!("Skipping mesh pass - disabled due to previous errors");
                        continue;
                    }
                    
                    self.execute_mesh_pass(encoder, frame_view, texture_pool, device, surface_width, surface_height, descriptor, &mut render_fn)
                        .map_err(|e| PassExecutionError::RenderingFailed(e.to_string()))
                }
                PassId::SlicePass => {
                    self.execute_slice_pass(encoder, frame_view, texture_pool, descriptor, &mut render_fn)
                        .map_err(|e| PassExecutionError::RenderingFailed(e.to_string()))
                }
                PassId::MipPass => {
                    self.execute_mip_pass(encoder, frame_view, texture_pool, descriptor, &mut render_fn)
                        .map_err(|e| PassExecutionError::RenderingFailed(e.to_string()))
                }
            };

            match &pass_result {
                Ok(_) => {
                    log::trace!("[FRAME_EXEC] {} pass '{}' completed successfully", pass_name, descriptor.name);
                }
                Err(error) => {
                    log::error!("[FRAME_EXEC] {} pass '{}' failed: {}", pass_name, descriptor.name, error);
                    frame_success = false;
                    self.handle_pass_error(pass_id, error.clone());
                    
                    // For slice pass errors, we still try to continue since 2D rendering is critical
                    if matches!(pass_id, PassId::SlicePass) {
                        log::error!("Critical slice pass failure - this may affect 2D rendering");
                        return Err(Box::new(PassExecutionError::RenderingFailed("Slice pass failed".to_string())));
                    }
                }
            }
        }

        if frame_success {
            self.record_success();
            log::trace!("[FRAME_EXEC] Frame execution completed successfully in {:.2}ms", 
                        frame_start_time.elapsed().as_millis_f64());
        } else {
            log::warn!("[FRAME_EXEC] Frame execution completed with errors in {:.2}ms", 
                       frame_start_time.elapsed().as_millis_f64());
        }

        Ok(())
    }

    /// Execute the mesh pass (direct to surface with depth)
    fn execute_mesh_pass<F>(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        frame_view: &wgpu::TextureView,
        texture_pool: &mut TexturePoolType,
        device: &wgpu::Device,
        surface_width: u32,
        surface_height: u32,
        descriptor: &PassDescriptor,
        render_fn: &mut F,
    ) -> Result<(), Box<dyn std::error::Error>>
    where
        F: FnMut(PassContext) -> Result<(), Box<dyn std::error::Error>>,
    {
        use crate::rendering::core::pipeline::get_mesh_depth_format;

        let start_time = Instant::now();
        log::trace!("[MESH_PASS] Starting execution - Pass: '{}', Size: {}x{}, Target: Surface (direct), Depth: {}", 
                    descriptor.name, surface_width, surface_height, descriptor.uses_depth);

        // Prepare depth texture if needed (only depth, no offscreen color texture)
        let depth_view_opt = if descriptor.uses_depth {
            let texture_start = Instant::now();
            texture_pool.ensure_textures(device, surface_width, surface_height, descriptor.uses_depth);
            log::trace!("[MESH_PASS] Depth texture preparation completed in {:.2}ms", 
                        texture_start.elapsed().as_millis_f64());

            let view_start = Instant::now();
            // Get just the depth view from get_mesh_views, ignoring the color view
            let (_, depth_view_opt) = texture_pool.get_mesh_views(
                device, 
                surface_width, 
                surface_height, 
                wgpu::TextureFormat::Bgra8UnormSrgb, 
                true
            );
            log::trace!("[MESH_PASS] Depth view acquisition completed in {:.2}ms", 
                        view_start.elapsed().as_millis_f64());
            depth_view_opt
        } else {
            None
        };
        
        // Log depth attachment status
        match &depth_view_opt {
            Some(_) => log::trace!("[MESH_PASS] Depth attachment: ENABLED (format: {:?})", 
                                   get_mesh_depth_format()),
            None => log::trace!("[MESH_PASS] Depth attachment: DISABLED - rendering without depth"),
        }

        // Render directly to surface view
        let render_pass_start = Instant::now();
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some(&descriptor.name),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: frame_view,  // Render directly to surface
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load, // Preserve existing content from previous passes (e.g., mesh rendering)
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: depth_view_opt.as_ref().map(|depth_view| {
                wgpu::RenderPassDepthStencilAttachment {
                    view: depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: if descriptor.clear_depth {
                            wgpu::LoadOp::Clear(1.0)
                        } else {
                            wgpu::LoadOp::Load
                        },
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }
            }),
            occlusion_query_set: None,
            timestamp_writes: None,
        });
        log::trace!("[MESH_PASS] Render pass creation completed in {:.2}ms", 
                    render_pass_start.elapsed().as_millis_f64());

        // Execute rendering
        let rendering_start = Instant::now();
        let context = PassContext::new(&mut render_pass, descriptor, PassId::MeshPass);
        let render_result = render_fn(context);
        let rendering_time = rendering_start.elapsed().as_millis_f64();

        match &render_result {
            Ok(_) => {
                log::trace!("[MESH_PASS] Rendering completed successfully in {:.2}ms", rendering_time);
                log::trace!("[MESH_PASS] Total execution time: {:.2}ms", 
                            start_time.elapsed().as_millis_f64());
            }
            Err(e) => {
                log::error!("[MESH_PASS] Rendering failed after {:.2}ms: {}", rendering_time, e);
            }
        }

        render_result
    }





    /// Execute the slice pass (onscreen without depth)
    fn execute_slice_pass<F>(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        frame_view: &wgpu::TextureView,
        texture_pool: &TexturePoolType,
        descriptor: &PassDescriptor,
        render_fn: &mut F,
    ) -> Result<(), Box<dyn std::error::Error>>
    where
        F: FnMut(PassContext) -> Result<(), Box<dyn std::error::Error>>,
    {
        let start_time = Instant::now();
        log::trace!("[SLICE_PASS] Starting execution - Pass: '{}', Target: Surface (onscreen)", 
                    descriptor.name);
        log::trace!("[SLICE_PASS] Clear color: {:?}, Clear depth: {}", 
                    descriptor.clear_color, descriptor.clear_depth);

        // Begin slice render pass (renders directly to surface, preserving mesh content)
        let render_pass_start = Instant::now();
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some(&descriptor.name),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: frame_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load, // Preserve existing mesh content
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None, // No depth for 2D slice rendering
            occlusion_query_set: None,
            timestamp_writes: None,
        });
        log::trace!("[SLICE_PASS] Render pass creation completed in {:.2}ms", 
                    render_pass_start.elapsed().as_millis_f64());
        log::trace!("[SLICE_PASS] Depth attachment: DISABLED (2D slice rendering)");

        // Execute rendering
        let rendering_start = Instant::now();
        let context = PassContext::new(&mut render_pass, descriptor, PassId::SlicePass);
        let render_result = render_fn(context);
        let rendering_time = rendering_start.elapsed().as_millis_f64();

        match &render_result {
            Ok(_) => {
                log::trace!("[SLICE_PASS] Rendering completed successfully in {:.2}ms", rendering_time);
                log::trace!("[SLICE_PASS] Total execution time: {:.2}ms", 
                            start_time.elapsed().as_millis_f64());
            }
            Err(e) => {
                log::error!("[SLICE_PASS] Rendering failed after {:.2}ms: {}", rendering_time, e);
            }
        }

        render_result
    }

    /// Execute the MIP pass (onscreen without depth)
    fn execute_mip_pass<F>(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        frame_view: &wgpu::TextureView,
        texture_pool: &TexturePoolType,
        descriptor: &PassDescriptor,
        render_fn: &mut F,
    ) -> Result<(), Box<dyn std::error::Error>>
    where
        F: FnMut(PassContext) -> Result<(), Box<dyn std::error::Error>>,
    {
        let start_time = Instant::now();
        log::trace!("[MIP_PASS] Starting execution - Pass: '{}', Target: Surface (onscreen)", 
                    descriptor.name);
        log::trace!("[MIP_PASS] Clear color: {:?}, Clear depth: {}", 
                    descriptor.clear_color, descriptor.clear_depth);

        // Begin MIP render pass (renders directly to surface)
        let render_pass_start = Instant::now();
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some(&descriptor.name),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: frame_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None, // No depth for MIP rendering
            occlusion_query_set: None,
            timestamp_writes: None,
        });
        log::trace!("[MIP_PASS] Render pass creation completed in {:.2}ms", 
                    render_pass_start.elapsed().as_millis_f64());
        log::trace!("[MIP_PASS] Depth attachment: DISABLED (MIP rendering)");

        // Execute rendering
        let rendering_start = Instant::now();
        let context = PassContext::new(&mut render_pass, descriptor, PassId::MipPass);
        let render_result = render_fn(context);
        let rendering_time = rendering_start.elapsed().as_millis_f64();

        match &render_result {
            Ok(_) => {
                log::trace!("[MIP_PASS] Rendering completed successfully in {:.2}ms", rendering_time);
                log::trace!("[MIP_PASS] Total execution time: {:.2}ms", 
                            start_time.elapsed().as_millis_f64());
            }
            Err(e) => {
                log::error!("[MIP_PASS] Rendering failed after {:.2}ms: {}", rendering_time, e);
            }
        }

        render_result
    }
}