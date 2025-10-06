//! Render pipeline management and utilities for wgpu.
//!
//! This module provides:
//! - A lightweight cache for `wgpu::RenderPipeline` objects keyed by `PipelineKey`.
//! - Helpers to create common pipelines (e.g., texture-quad and mesh) and uniform bind groups.
//! - A global swapchain/surface format accessor to ensure pipelines target the correct color format.
//!
//! Design notes:
//! - Pipeline creation is relatively expensive. Caching pipelines avoids redundant compilation work and
//!   improves frame-time stability, especially in interactive contexts (native and WASM).
//! - Keys aim to be deterministic across runs and platforms. Current signatures use string serialization;
//!   consider migrating to a compact hash for performance.
//! - This module does not manage shader hot-reload; shader modules are compiled at creation time.
//!
//! Invariants:
//! - Bind group layout order for the texture quad pipeline is [texture, vertex uniforms, fragment uniforms].
//! - Vertex buffer layouts used for cache keys must reflect all vertex attributes that influence pipeline creation.
//! - Pipelines created here are valid for the device they are created from and must not be shared across devices.
//!
//! TODO:
//! - Replace string-based signatures with a stable hash for faster keying.
//! - Add depth-stencil configuration options for pipelines that require ordering or testing.
//! - Introduce feature flags for toggling debug trace logs and pipeline instrumentation.
//! - Consider adding shader module caching or a simple hot-reload mechanism in dev builds.
//! - Add tests covering cache hit/miss behavior and signature correctness.
#![allow(dead_code)]

use std::collections::HashMap;
use wgpu::util::DeviceExt;
use once_cell::sync::OnceCell;
use std::sync::Arc;

/// Global storage for the swapchain/surface color target format used by onscreen render passes.
///
/// Notes
/// - Some pipelines depend on the target color format (e.g., blending and write mask behavior).
/// - Storing it here avoids threading a format parameter throughout the call graph when a single
///   surface is used. On multi-surface setups, prefer passing the format explicitly.
static SWAPCHAIN_FORMAT: OnceCell<wgpu::TextureFormat> = OnceCell::new();

/// Sets the global swapchain/surface color target format. Safe to call multiple times; subsequent calls will be ignored.
///
/// Parameters
/// - `fmt`: The `wgpu::TextureFormat` used by the surface/swapchain color attachment.
///
/// Notes
/// - Uses `OnceCell` so the first successful set wins; later attempts are no-ops.
/// - Prefer explicit formats for multi-window/surface scenarios instead of relying on global state.
pub fn set_swapchain_format(fmt: wgpu::TextureFormat) {
    // No-op if already set; `OnceCell` ensures thread-safe one-time initialization.
    let _ = SWAPCHAIN_FORMAT.set(fmt);
}

/// Gets the globally stored swapchain/surface color target format, if set.
///
/// Returns
/// - `Option<wgpu::TextureFormat>`: `Some(format)` if previously set, otherwise `None`.
pub fn get_swapchain_format() -> Option<wgpu::TextureFormat> {
    // Surfaces may negotiate formats with the backend; ensure this is set during initialization
    // after surface configuration. Call sites can fall back to reasonable defaults when `None`.
    SWAPCHAIN_FORMAT.get().copied()
}

/// Minimal pipeline cache key for render pipelines.
///
/// Keys group pipelines by characteristics that affect pipeline creation. This ensures we reuse
/// identical pipelines rather than re-compiling them. Keys should be deterministic across builds.
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub enum PipelineKey {
    /// Volume slice quad pipeline, parameterized by the target surface format and layout signatures.
    VolumeSliceQuad { target_format: wgpu::TextureFormat, vertex_sig: String, bgl_sig: String },
    /// Basic mesh pipeline, parameterized by target format and primitive topology.
    MeshBasic { target_format: wgpu::TextureFormat, topology: wgpu::PrimitiveTopology },
    /// Custom pipelines keyed by a deterministic descriptor signature.
    Custom { signature: String },
}

/// Pipeline manager for caching and retrieving `wgpu::RenderPipeline` objects.
///
/// Responsibilities
/// - Maintain a map of pipelines keyed by `PipelineKey`.
/// - Provide hit/miss counters for basic cache instrumentation.
/// - Offer helper operations to insert, clear, and snapshot keys.
///
/// Concurrency
/// - This manager is not thread-safe and expects exclusive `&mut` access. If you need concurrent
///   access, consider `Arc<Mutex<PipelineManager>>` or redesign to use atomics for counters and
///   immutable values for pipeline storage.
pub struct PipelineManager {
    /// Cache storage keyed by pipeline descriptors that influence pipeline creation.
    pipelines: HashMap<PipelineKey, Arc<wgpu::RenderPipeline>>, 
    /// Total number of cache hits observed.
    hit_count: usize,
    /// Total number of cache misses observed.
    miss_count: usize,
}

impl PipelineManager {
    /// Creates a new, empty pipeline manager.
    pub fn new() -> Self {
        Self { pipelines: HashMap::new(), hit_count: 0, miss_count: 0 }
    }

    /// Returns a reference to a cached pipeline by key if it exists.
    ///
    /// Parameters
    /// - `key`: The `PipelineKey` to look up.
    ///
    /// Returns
    /// - `Option<&Arc<wgpu::RenderPipeline>>`: Reference to the cached pipeline if present.
    pub fn get(&self, key: &PipelineKey) -> Option<&Arc<wgpu::RenderPipeline>> {
        self.pipelines.get(key)
    }

    /// Inserts or replaces a pipeline under the given key.
    ///
    /// Parameters
    /// - `key`: Key describing the pipeline.
    /// - `pipeline`: The `wgpu::RenderPipeline` wrapped in `Arc` to allow shared ownership.
    ///
    /// Notes
    /// - Replacing an existing pipeline under the same key will overwrite the previous value.
    pub fn insert(&mut self, key: PipelineKey, pipeline: Arc<wgpu::RenderPipeline>) {
        log::info!("Inserted pipeline {:?}", &key);
        self.pipelines.insert(key, pipeline);
    }

    /// Removes a pipeline from the cache and returns it if present.
    pub fn remove(&mut self, key: &PipelineKey) -> Option<Arc<wgpu::RenderPipeline>> {
        self.pipelines.remove(key)
    }

    /// Clears all cached pipelines and resets counters.
    ///
    /// Notes
    /// - Clearing pipelines invalidates GPU state references. Ensure no active render passes rely on
    ///   these pipelines at the time of clearing.
    pub fn clear(&mut self) {
        self.pipelines.clear();
        self.hit_count = 0;
        self.miss_count = 0;
    }

    /// Checks if a pipeline with the given key exists.
    pub fn exists(&self, key: &PipelineKey) -> bool {
        self.pipelines.contains_key(key)
    }

    /// Returns current cache size.
    pub fn cache_size(&self) -> usize { self.pipelines.len() }

    /// Clears all cached pipelines. Alias for `clear()`.
    pub fn invalidate_all(&mut self) {
        self.clear();
    }

    /// Records a cache hit in the manager's monitoring counters.
    pub fn record_hit(&mut self) { self.hit_count += 1; }

    /// Records a cache miss in the manager's monitoring counters.
    pub fn record_miss(&mut self) { self.miss_count += 1; }

    /// Returns the total number of cache hits observed since initialization.
    pub fn hits(&self) -> usize { self.hit_count }

    /// Returns the total number of cache misses observed since initialization.
    pub fn misses(&self) -> usize { self.miss_count }

    /// Returns a snapshot of the current set of pipeline keys in the cache.
    pub fn keys_snapshot(&self) -> Vec<PipelineKey> { self.pipelines.keys().cloned().collect() }
}

// Removed global PIPELINE_MANAGER singleton to support WASM and instance-based management.

/// Returns a cached texture-quad pipeline if present, otherwise creates, caches, and returns it.
///
/// Parameters
/// - `manager`: Pipeline cache used to deduplicate `wgpu::RenderPipeline` creation.
/// - `device`: Logical device used for pipeline creation.
/// - `bind_group_layouts`: Trio of bind group layouts in order [texture, vertex uniforms, fragment uniforms].
/// - `vertex_buffers`: Vertex buffer layouts describing the quad vertex input.
/// - `target_format`: Color target format; typically the swapchain/surface format.
///
/// Returns
/// - `Arc<wgpu::RenderPipeline>`: Shared reference to the cached or newly created pipeline.
///
/// Notes
/// - Keying uses string-based signatures of vertex layouts and bind group layouts to ensure stability across runs.
/// - Hit/miss counters are updated for simple cache instrumentation.
///
/// TODO
/// - Replace string signatures with a compact, stable hash (e.g., `ahash`) to reduce key memory and compare time.
/// - Support MSAA and depth-stencil variants via extended `PipelineKey` fields.
pub fn get_or_create_texture_quad_pipeline(
    manager: &mut PipelineManager,
    device: &wgpu::Device,
    bind_group_layouts: [&wgpu::BindGroupLayout; 3],
    vertex_buffers: &[wgpu::VertexBufferLayout<'static>],
    target_format: wgpu::TextureFormat,
) -> Arc<wgpu::RenderPipeline> {
    // Compute signatures for cache key stability. These reflect inputs that influence pipeline creation.
    let vertex_sig = vertex_layout_signature(vertex_buffers);
    let bgl_sig = default_slice_bgl_signature();

    // Compose the cache key from target format and layout signatures.
    let key = PipelineKey::VolumeSliceQuad { target_format, vertex_sig: vertex_sig.clone(), bgl_sig: bgl_sig.clone() };
    if let Some(p) = { manager.get(&key).cloned() } {
        manager.hit_count += 1;
        if cfg!(feature = "pipeline_debug") {
            // Verbose tracing is gated behind a feature to avoid log noise in production builds.
            log::trace!(
                "Pipeline cache hit: {:?}. Hits={}, Misses={}, Size={}",
                key,
                manager.hit_count,
                manager.miss_count,
                manager.cache_size()
            );
        }
        return p;
    }

    if cfg!(feature = "pipeline_debug") {
        log::trace!("Pipeline cache miss: {:?}. Creating.", key);
    }
    let pipeline = create_texture_quad_pipeline(device, bind_group_layouts, vertex_buffers, target_format);
    let pipeline = Arc::new(pipeline);
    manager.miss_count += 1;
    manager.insert(key, pipeline.clone());
    if cfg!(feature = "pipeline_debug") {
        log::trace!(
            "Pipeline inserted. Hits={}, Misses={}, Size={}",
            manager.hit_count,
            manager.miss_count,
            manager.cache_size()
        );
    }
    pipeline
}

/// Creates the texture-quad render pipeline used by 2D MPR views.
/// Expects three bind group layouts: texture, vertex uniforms, fragment uniforms.
///
/// Parameters
/// - `device`: Device used to create shader modules, layouts, and pipelines.
/// - `bind_group_layouts`: Array of [texture, vertex uniform, fragment uniform] layouts used by the shader.
/// - `vertex_buffers`: Quad vertex buffer layouts.
/// - `target_format`: Color target format for the render pass (swapchain/surface format).
///
/// Returns
/// - `wgpu::RenderPipeline`: Newly created pipeline. Callers are expected to cache it via `PipelineManager`.
///
/// Notes
/// - Pipeline state is fully specified for deterministic behavior across platforms.
/// - Blend state is set to REPLACE; adjust if alpha blending is desired.
///
/// TODO
/// - Parameterize MSAA (`multisample.count`) and culling for performance/quality trade-offs.
/// - Add optional depth-stencil to support ordered rendering or Z-tests.
pub fn create_texture_quad_pipeline(
    device: &wgpu::Device,
    bind_group_layouts: [&wgpu::BindGroupLayout; 3],
    vertex_buffers: &[wgpu::VertexBufferLayout<'static>],
    target_format: wgpu::TextureFormat,
) -> wgpu::RenderPipeline {
    // Single shader module with both vertex and fragment entry points.
    let shader = device.create_shader_module(wgpu::include_wgsl!("shader/shader_tex.wgsl"));
    // Pipeline layout defines bind group layout order; must match shader binding expectations.
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Render Pipeline Layout"),
        bind_group_layouts: &bind_group_layouts,
        push_constant_ranges: &[],
    });

    // Full pipeline descriptor. All fields annotated for clarity.
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Render Pipeline"),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"), // WGSL entry point for vertex stage
            buffers: vertex_buffers,        // Vertex buffer layouts (position, texcoord, etc.)
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs_main"), // WGSL entry point for fragment stage
            targets: &[Some(wgpu::ColorTargetState {
                format: target_format,             // Target color format (swapchain surface)
                blend: Some(wgpu::BlendState::REPLACE), // No blending; write replaces previous value
                write_mask: wgpu::ColorWrites::ALL,     // Write all color channels
            })],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList, // Quad rendered as two triangles
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: None,                    // No face culling; adjust for performance if needed
            polygon_mode: wgpu::PolygonMode::Fill,
            unclipped_depth: false,
            conservative: false,
        },
        depth_stencil: None, // No depth testing; suitable for 2D overlays
        multisample: wgpu::MultisampleState {
            count: 1,                          // No MSAA; parameterize for quality improvements
            mask: !0,
            alpha_to_coverage_enabled: false,
        },
        multiview: None,
        cache: None,
    })
}

/// Returns a cached basic mesh pipeline (no bind groups) if present, otherwise creates, caches, and returns it.
/// Uses global swapchain format if set; otherwise falls back to Rgba8Unorm. Keyed by target format and topology.
///
/// Parameters
/// - `manager`: Pipeline cache used to deduplicate creation.
/// - `device`: Logical device used to build shader modules and pipelines.
///
/// Returns
/// - `Arc<wgpu::RenderPipeline>`: Shared pipeline handle for mesh rendering.
///
/// Notes
/// - This pipeline has no bind groups and renders points using `MeshVertex::desc()` layout.
/// - Uses REPLACE blending; adjust if you need alpha.
///
/// TODO
/// - Parameterize topology and pipeline state (cull mode, MSAA, depth) via input args or `PipelineKey`.
/// - Add bind groups for uniforms and textures as needed.
#[cfg(feature = "mesh")]
pub fn get_or_create_mesh_pipeline(manager: &mut PipelineManager, device: &wgpu::Device) -> Arc<wgpu::RenderPipeline> {
    let target_format = get_swapchain_format().unwrap_or(wgpu::TextureFormat::Rgba8Unorm);
    let topology = wgpu::PrimitiveTopology::PointList;
    let key = PipelineKey::MeshBasic { target_format, topology };

    if let Some(p) = { manager.get(&key).cloned() } {
        manager.hit_count += 1;
        log::trace!(
            "Pipeline cache hit: {:?}. Hits={}, Misses={}, Size={}",
            key,
            manager.hit_count,
            manager.miss_count,
            manager.cache_size()
        );
        return p;
    }

    log::trace!("Pipeline cache miss: {:?}. Creating.", key);
    // Mesh shader with both vertex and fragment stages.
    let shader = device.create_shader_module(wgpu::include_wgsl!("shader/mesh.wgsl"));
    // No bind groups for this basic pipeline; layout is empty.
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Mesh Pipeline Layout"),
        bind_group_layouts: &[],
        push_constant_ranges: &[],
    });
    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Mesh Pipeline"),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            buffers: &[crate::mesh::mesh::MeshVertex::desc()],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs_main"),
            targets: &[Some(wgpu::ColorTargetState {
                format: target_format,
                blend: Some(wgpu::BlendState::REPLACE),
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        }),
        primitive: wgpu::PrimitiveState {
            topology,
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: None,
            polygon_mode: wgpu::PolygonMode::Fill,
            unclipped_depth: false,
            conservative: false,
        },
        depth_stencil: None,
        multisample: wgpu::MultisampleState {
            count: 1,
            mask: !0,
            alpha_to_coverage_enabled: false,
        },
        multiview: None,
        cache: None,
    });

    let pipeline = Arc::new(pipeline);
    manager.miss_count += 1;
    manager.insert(key, pipeline.clone());
    log::trace!(
        "Pipeline inserted. Hits={}, Misses={}, Size={}",
        manager.hit_count,
        manager.miss_count,
        manager.cache_size()
    );
    pipeline
}

/// Computes a deterministic signature string for the provided vertex buffer layouts.
///
/// Parameters
/// - `layouts`: Slice of vertex buffer layouts used by a pipeline.
///
/// Returns
/// - `String`: A stable string encoding key properties of the layouts (stride, step mode, attributes).
///
/// Notes
/// - This is used to strengthen cache keys for TextureQuad by reflecting vertex input configuration.
/// - The format is textual for readability; switching to a hash can reduce memory and speed comparisons.
///
/// TODO
/// - Use a hasher (e.g., `ahash`) to produce a compact signature instead of a string.
pub fn vertex_layout_signature(layouts: &[wgpu::VertexBufferLayout<'static>]) -> String {
    let mut s = String::new();
    s.push_str(&format!("count:{};", layouts.len()));
    for (i, vb) in layouts.iter().enumerate() {
        s.push_str(&format!("i:{};stride:{};step:{:?};attrs:{};", i, vb.array_stride, vb.step_mode, vb.attributes.len()));
        for a in vb.attributes.iter() {
            s.push_str(&format!("loc:{};off:{};fmt:{:?};", a.shader_location, a.offset, a.format));
        }
    }
    s
}

/// Returns a static signature representing the default trio of bind group layouts
/// used by RenderContext for texture rendering.
///
/// Returns
/// - `String`: Signature used as part of cache keys for the texture quad pipeline.
pub fn default_slice_bgl_signature() -> String {
    "texture+vertex+fragment_uniforms_v1".to_string()
}

/// Creates a uniform buffer and a bind group for a given layout and data.
///
/// Parameters
/// - `device`: Logical device used to allocate buffer and bind group.
/// - `layout`: Bind group layout describing the uniform binding at slot 0.
/// - `data`: Uniform POD data to upload.
///
/// Returns
/// - `(wgpu::Buffer, wgpu::BindGroup)`: The created uniform buffer and its bind group.
///
/// Notes
/// - Buffer usage includes UNIFORM and COPY_DST to allow updates via queue.write_buffer.
/// - Binding uses offset 0 and unspecified size (entire buffer).
///
/// TODO
/// - Consider aligning buffer size to `device.limits().min_uniform_buffer_offset_alignment` when using dynamic offsets.
pub fn create_uniform_bind_group<T: bytemuck::Pod>(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    data: &T,
) -> (wgpu::Buffer, wgpu::BindGroup) {
    let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Uniform Buffer"),
        contents: bytemuck::cast_slice(&[*data]),
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    });

    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                buffer: &buffer,
                offset: 0,
                size: None,
            }),
        }],
        label: Some("Uniform Bind Group"),
    });

    (buffer, bind_group)
}

/// Creates vertex-stage uniform bind group and layout.
///
/// Parameters
/// - `device`: Logical device used to allocate resources.
/// - `data`: Vertex-stage uniform POD data.
///
/// Returns
/// - `(wgpu::Buffer, wgpu::BindGroup, wgpu::BindGroupLayout)`: The buffer, bind group, and layout.
///
/// Notes
/// - Layout visibility is restricted to the VERTEX stage.
///
/// TODO
/// - Expose `has_dynamic_offset` and `min_binding_size` for advanced uniform management.
pub fn create_vertex_uniform_bind_group<T: bytemuck::Pod>(
    device: &wgpu::Device,
    data: &T,
) -> (wgpu::Buffer, wgpu::BindGroup, wgpu::BindGroupLayout) {
    let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::VERTEX,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        }],
        label: Some("uniform_bind_group_layout"),
    });

    let (buffer, bind_group) = create_uniform_bind_group(device, &layout, data);
    (buffer, bind_group, layout)
}

/// Creates fragment-stage uniform bind group and layout.
///
/// Parameters
/// - `device`: Logical device used to allocate resources.
/// - `data`: Fragment-stage uniform POD data.
///
/// Returns
/// - `(wgpu::Buffer, wgpu::BindGroup, wgpu::BindGroupLayout)`: The buffer, bind group, and layout.
///
/// Notes
/// - Layout visibility is restricted to the FRAGMENT stage.
///
/// TODO
/// - Expose `has_dynamic_offset` and `min_binding_size` for advanced uniform management.
pub fn create_fragment_uniform_bind_group<T: bytemuck::Pod>(
    device: &wgpu::Device,
    data: &T,
) -> (wgpu::Buffer, wgpu::BindGroup, wgpu::BindGroupLayout) {
    let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        }],
        label: Some("uniform_frag_bind_group_layout"),
    });
    let (buffer, bind_group) = create_uniform_bind_group(device, &layout, data);
    (buffer, bind_group, layout)
}