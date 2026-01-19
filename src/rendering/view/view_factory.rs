//! ViewFactory trait separated for cleaner module boundaries
//!
//! This file defines the ViewFactory trait, extracted from view.rs to decouple
//! factory responsibilities from core view types. Keeping the factory in its own
//! source file improves testability and future extensibility (e.g., different
//! factory implementations for native and WebAssembly builds).

use super::{Orientation, View};
use crate::data::volume_encoding::VolumeEncoding;
use crate::CTVolume;

use log::{debug, info};
use std::sync::Arc;

use crate::core::WindowLevel;
use crate::rendering::view::mesh::basic_mesh_context::BasicMeshContext;
use crate::rendering::view::mesh::mesh::Mesh;
use crate::rendering::view::mesh::mesh_view::MeshView;
use crate::rendering::view::mip::{MipView, MipViewWgpuImpl};
use crate::rendering::view::mpr::mpr_render_context::MprRenderContext;
use crate::rendering::view::mpr::mpr_view::MprView;
use crate::rendering::view::render_content::RenderContent;

/// Factory trait for creating different types of views.
///
/// Centralizes view creation logic and provides a consistent interface for
/// creating views with proper initialization parameters. This pattern ensures
/// that all views are created with the correct dependencies and configuration.
///
/// Benefits:
/// - Consistent view initialization across the application
/// - Centralized dependency injection for view creation
/// - Easy testing through mock factory implementations
/// - Type-safe view creation with proper error handling
pub trait ViewFactory {
    /// Create a new mesh view with specified position and dimensions.
    ///
    /// Returns a boxed View trait object ready for rendering 3D mesh data.
    /// The view will be configured for 3D visualization with appropriate
    /// camera settings and rendering pipeline.
    fn create_mesh_view(
        &self,
        mesh: &Mesh,
        pos: (i32, i32),
        size: (u32, u32),
    ) -> Result<Box<dyn View>, Box<dyn std::error::Error>>;

    /// Create a new MPR view with volume data and orientation.
    ///
    /// Returns a boxed View trait object configured for medical imaging display.
    /// The view will be set up with appropriate shaders, uniforms, and geometry
    /// for the specified anatomical orientation.
    ///
    /// Parameters:
    /// - `vol`: CT volume data containing the medical imaging dataset
    /// - `orientation`: Anatomical orientation (Transverse, Coronal, Sagittal, Oblique)
    /// - `pos`: Initial position on screen
    /// - `size`: Initial dimensions of the view
    fn create_mpr_view(
        &self,
        vol: &CTVolume,
        orientation: Orientation,
        pos: (i32, i32),
        size: (u32, u32),
    ) -> Result<Box<dyn View>, Box<dyn std::error::Error>>;

    /// Create a new MIP (Maximum Intensity Projection) view.
    ///
    /// Returns a boxed View configured for MIP rendering of the provided CT volume.
    /// The view will be set up with appropriate uniforms and pipeline for MIP.
    ///
    /// Parameters:
    /// - `vol`: CT volume data to be visualized via MIP
    /// - `pos`: Initial position on screen
    /// - `size`: Initial dimensions of the view
    fn create_mip_view(
        &self,
        vol: &CTVolume,
        pos: (i32, i32),
        size: (u32, u32),
    ) -> Result<Box<dyn View>, Box<dyn std::error::Error>>;

    /// Function-level comment: Create an MPR view using a prebuilt RenderContent for GPU reuse
    ///
    /// This variant allows callers to provide an Arc<RenderContent> so that multiple views
    /// can share the same 3D texture without re-uploading volume data. The CTVolume reference
    /// is still required to provide metadata (dimensions/spacing) for correct slicing math.
    fn create_mpr_view_with_content(
        &self,
        render_content: Arc<RenderContent>,
        vol: &CTVolume,
        orientation: Orientation,
        pos: (i32, i32),
        size: (u32, u32),
    ) -> Result<Box<dyn View>, Box<dyn std::error::Error>>;

    /// Function-level comment: Create a MIP view using a prebuilt RenderContent for GPU reuse
    ///
    /// This variant allows callers to provide an Arc<RenderContent> so that multiple views
    /// can share the same 3D texture without re-uploading. This is ideal when building MPR
    /// and MIP views for the same volume concurrently.
    fn create_mip_view_with_content(
        &self,
        render_content: Arc<RenderContent>,
        pos: (i32, i32),
        size: (u32, u32),
    ) -> Result<Box<dyn View>, Box<dyn std::error::Error>>;

    /// Create a Mesh view using a prebuilt RenderContent for API consistency
    /// RenderContent is not used by mesh rendering but allows unified factory usage.
    fn create_mesh_view_with_content(
        &self,
        render_content: Arc<RenderContent>,
        mesh: &Mesh,
        pos: (i32, i32),
        size: (u32, u32),
    ) -> Result<Box<dyn View>, Box<dyn std::error::Error>>;
}

pub struct MockViewFactory;

impl ViewFactory for MockViewFactory {
    /// Function-level comment: Mesh view creation stub returning an error for test scenarios
    fn create_mesh_view(
        &self,
        _mesh: &Mesh,
        _pos: (i32, i32),
        _size: (u32, u32),
    ) -> Result<Box<dyn View>, Box<dyn std::error::Error>> {
        Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::Other,
            "Mock factory - not implemented",
        )))
    }

    /// Function-level comment: MPR view creation stub returning an error for test scenarios
    fn create_mpr_view(
        &self,
        _vol: &CTVolume,
        _orientation: Orientation,
        _pos: (i32, i32),
        _size: (u32, u32),
    ) -> Result<Box<dyn View>, Box<dyn std::error::Error>> {
        Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::Other,
            "Mock factory - not implemented",
        )))
    }

    /// Function-level comment: MIP view creation stub returning an error for test scenarios
    fn create_mip_view(
        &self,
        _vol: &CTVolume,
        _pos: (i32, i32),
        _size: (u32, u32),
    ) -> Result<Box<dyn View>, Box<dyn std::error::Error>> {
        Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::Other,
            "Mock factory - not implemented",
        )))
    }

    /// Function-level comment: MPR with content stub returning an error for test scenarios
    fn create_mpr_view_with_content(
        &self,
        _render_content: Arc<RenderContent>,
        _vol: &CTVolume,
        _orientation: Orientation,
        _pos: (i32, i32),
        _size: (u32, u32),
    ) -> Result<Box<dyn View>, Box<dyn std::error::Error>> {
        Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::Other,
            "Mock factory - not implemented",
        )))
    }

    /// Function-level comment: MIP with content stub returning an error for test scenarios
    fn create_mip_view_with_content(
        &self,
        _render_content: Arc<RenderContent>,
        _pos: (i32, i32),
        _size: (u32, u32),
    ) -> Result<Box<dyn View>, Box<dyn std::error::Error>> {
        Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::Other,
            "Mock factory - not implemented",
        )))
    }

    fn create_mesh_view_with_content(
        &self,
        _render_content: Arc<RenderContent>,
        _mesh: &Mesh,
        _pos: (i32, i32),
        _size: (u32, u32),
    ) -> Result<Box<dyn View>, Box<dyn std::error::Error>> {
        Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::Other,
            "Mock factory - not implemented",
        )))
    }
}

/// Function-level comment: Concrete ViewFactory implementation that wires real GPU-backed views
///
/// DefaultViewFactory centralizes the creation of Mesh, MPR, and MIP views.
/// It owns the required WGPU device/queue references and surface format so that
/// all views are initialized consistently. Volume textures are constructed on demand
/// from CTVolume voxel data following either packed RG8 or native R16Float paths.
pub struct DefaultViewFactory {
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,
    surface_format: wgpu::TextureFormat,
    use_float_volume_texture: bool,
}

impl DefaultViewFactory {
    /// Function-level comment: Create a new DefaultViewFactory with required GPU resources and configuration
    pub fn new(
        device: Arc<wgpu::Device>,
        queue: Arc<wgpu::Queue>,
        surface_format: wgpu::TextureFormat,
        use_float_volume_texture: bool,
    ) -> Self {
        Self {
            device,
            queue,
            surface_format,
            use_float_volume_texture,
        }
    }

    /// Function-level comment: Build RenderContent from CTVolume using configured texture format path
    fn build_render_content(
        &self,
        vol: &CTVolume,
    ) -> Result<Arc<RenderContent>, Box<dyn std::error::Error>> {
        if self.use_float_volume_texture {
            debug!("[DefaultViewFactory] Using R16Float volume texture path");
            // Convert voxel i16 to half-float (f16) bit pattern then cast to bytes
            let bytes: Vec<u8> = {
                let voxels_f16_bits: Vec<u16> = vol
                    .voxel_data
                    .iter()
                    .map(|&x| half::f16::from_f32(x as f32).to_bits())
                    .collect();
                bytemuck::cast_slice(&voxels_f16_bits).to_vec()
            };
            let encoding = VolumeEncoding::HuFloat;
            match RenderContent::from_bytes_r16f(
                &self.device,
                &self.queue,
                &bytes,
                "CT Volume",
                vol.dimensions.0 as u32,
                vol.dimensions.1 as u32,
                vol.dimensions.2 as u32,
                encoding,
            ) {
                Ok(rc) => Ok(Arc::new(rc)),
                Err(e) => Err(Box::new(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    e.to_string(),
                ))),
            }
        } else {
            debug!("[DefaultViewFactory] Using Rg8Unorm volume texture path");
            let offset = VolumeEncoding::DEFAULT_HU_OFFSET;
            let voxel_data: Vec<u16> = vol
                .voxel_data
                .iter()
                .map(|x| (*x + offset as i16) as u16)
                .collect();
            let voxel_bytes: Vec<u8> = bytemuck::cast_slice(&voxel_data).to_vec();
            let encoding = VolumeEncoding::HuPackedRg8 { offset };
            match RenderContent::from_bytes(
                &self.device,
                &self.queue,
                &voxel_bytes,
                "CT Volume",
                vol.dimensions.0 as u32,
                vol.dimensions.1 as u32,
                vol.dimensions.2 as u32,
                encoding,
            ) {
                Ok(rc) => Ok(Arc::new(rc)),
                Err(e) => Err(Box::new(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    e.to_string(),
                ))),
            }
        }
    }
}

impl ViewFactory for DefaultViewFactory {
    /// Function-level comment: Create a MeshView with a fresh BasicMeshContext and default rotation enabled
    fn create_mesh_view(
        &self,
        mesh: &Mesh,
        pos: (i32, i32),
        size: (u32, u32),
    ) -> Result<Box<dyn View>, Box<dyn std::error::Error>> {
        use crate::rendering::view::View as _;

        let mut mesh_view = MeshView::new();
        mesh_view.set_rotation_enabled(false);
        info!("[DefaultViewFactory] Mesh rotation enabled");

        // Create fresh BasicMeshContext for each mesh view
        let ctx = BasicMeshContext::new(
            &self.device,
            &self.queue,
            mesh,
            true, // Enable depth testing for proper 3D rendering
        );
        let ctx_arc = Arc::new(ctx);

        mesh_view.attach_context(ctx_arc);
        mesh_view.move_to(pos);
        mesh_view.resize(size);
        Ok(Box::new(mesh_view))
    }

    /// Function-level comment: Create an MPR view configured for the requested orientation
    fn create_mpr_view(
        &self,
        vol: &CTVolume,
        orientation: Orientation,
        pos: (i32, i32),
        size: (u32, u32),
    ) -> Result<Box<dyn View>, Box<dyn std::error::Error>> {
        let render_content = match self.build_render_content(vol) {
            Ok(rc) => rc,
            Err(e) => return Err(e),
        };

        // Shared render context for MPR views
        let render_context = Arc::new(MprRenderContext::new(&self.device));

        // Configure WindowLevel defaults; mirror State logic where appropriate
        let mut winlev = WindowLevel::new();
        let decode_params = render_content.decode_parameters();
        if decode_params.bias != 0.0 {
            let _ = winlev.set_bias(decode_params.bias);
        }
        let _ = winlev.apply_bone_preset();

        let view = MprView::new(
            render_context,
            &self.device,
            render_content,
            vol,
            orientation,
            winlev,
            1.0,
            [0.0, 0.0, 0.0],
            pos,
            size,
        );

        info!(
            "[DefaultViewFactory] Created MPR view for {:?} at {:?} size {:?}",
            orientation, pos, size
        );
        Ok(Box::new(view))
    }

    /// Function-level comment: Create a MIP view sharing existing RenderContent for zero-copy volume access
    fn create_mip_view(
        &self,
        vol: &CTVolume,
        pos: (i32, i32),
        size: (u32, u32),
    ) -> Result<Box<dyn View>, Box<dyn std::error::Error>> {
        use crate::rendering::view::View as _;

        let render_content = match self.build_render_content(vol) {
            Ok(rc) => rc,
            Err(e) => return Err(e),
        };
        let mip_wgpu_impl = MipViewWgpuImpl::new(render_content, &self.device, self.surface_format);

        let mut mip_view = MipView::new(Arc::new(mip_wgpu_impl));
        mip_view.move_to(pos);
        mip_view.resize(size);

        info!(
            "[DefaultViewFactory] Created MIP view at {:?} size {:?}",
            pos, size
        );
        Ok(Box::new(mip_view))
    }

    /// Function-level comment: Create an MPR view using provided RenderContent for zero-copy reuse
    fn create_mpr_view_with_content(
        &self,
        render_content: Arc<RenderContent>,
        vol: &CTVolume,
        orientation: Orientation,
        pos: (i32, i32),
        size: (u32, u32),
    ) -> Result<Box<dyn View>, Box<dyn std::error::Error>> {
        // Shared render context for MPR views
        let render_context = Arc::new(MprRenderContext::new(&self.device));

        // Configure WindowLevel defaults; mirror State logic where appropriate
        let mut winlev = WindowLevel::new();
        let decode_params = render_content.decode_parameters();
        if decode_params.bias != 0.0 {
            let _ = winlev.set_bias(decode_params.bias);
        }
        let _ = winlev.apply_bone_preset();

        let view = MprView::new(
            render_context,
            &self.device,
            render_content,
            vol,
            orientation,
            winlev,
            1.0,
            [0.0, 0.0, 0.0],
            pos,
            size,
        );

        info!(
            "[DefaultViewFactory] Created MPR view (with_content) for {:?} at {:?} size {:?}",
            orientation, pos, size
        );
        Ok(Box::new(view))
    }

    /// Function-level comment: Create a MIP view using provided RenderContent for zero-copy reuse
    fn create_mip_view_with_content(
        &self,
        render_content: Arc<RenderContent>,
        pos: (i32, i32),
        size: (u32, u32),
    ) -> Result<Box<dyn View>, Box<dyn std::error::Error>> {
        use crate::rendering::view::View as _;

        let mip_wgpu_impl = MipViewWgpuImpl::new(render_content, &self.device, self.surface_format);

        let mut mip_view = MipView::new(Arc::new(mip_wgpu_impl));
        mip_view.move_to(pos);
        mip_view.resize(size);

        info!(
            "[DefaultViewFactory] Created MIP view (with_content) at {:?} size {:?}",
            pos, size
        );
        Ok(Box::new(mip_view))
    }

    fn create_mesh_view_with_content(
        &self,
        _render_content: Arc<RenderContent>,
        mesh: &Mesh,
        pos: (i32, i32),
        size: (u32, u32),
    ) -> Result<Box<dyn View>, Box<dyn std::error::Error>> {
        use crate::rendering::view::View as _;

        let mut mesh_view = MeshView::new();
        mesh_view.set_rotation_enabled(false);
        info!("[DefaultViewFactory] Mesh rotation disabled for consistent inspection");

        let ctx = BasicMeshContext::new(&self.device, &self.queue, mesh, true);
        let ctx_arc = Arc::new(ctx);

        mesh_view.attach_context(ctx_arc);
        mesh_view.move_to(pos);
        mesh_view.resize(size);

        // Initialize and attach orientation cube context (same as create_mesh_view)
        let cube_mesh = crate::rendering::mesh::mesh::Mesh::unit_cube();
        let cube_ctx = BasicMeshContext::new(&self.device, &self.queue, &cube_mesh, true);
        mesh_view.attach_orientation_cube_context(Arc::new(cube_ctx));

        info!(
            "[DefaultViewFactory] Created Mesh view (with_content) at {:?} size {:?}",
            pos, size
        );
        Ok(Box::new(mesh_view))
    }
}
