Summary
Added with_content variants to ViewFactory for MPR and MIP views, enabling reuse of a prebuilt Arc<RenderContent> across multiple views. This avoids repeated volume conversions and GPU uploads, improving performance and memory efficiency.

Design notes
- The existing CTVolume-based factory methods remain for convenience and backward compatibility.
- The with_content variants accept Arc<RenderContent>, allowing callers to prebuild the 3D texture once (e.g., background task, caching layer) and reuse it for MPR/MIP.
- MPR still requires a &CTVolume to access metadata (dimensions/spacing) for correct slicing math.
- The RenderContent carries the texture_format (R16Float or Rg8Unorm); pipelines support both paths.
- Logging uses INFO/DEBUG and is compatible with native (RUST_LOG) and wasm (console_log).

API additions
- create_mpr_view_with_content(&self, render_content: Arc<RenderContent>, vol: &CTVolume, orientation: Orientation, pos: (i32, i32), size: (u32, u32)) -> Result<Box<dyn View>, Box<dyn Error>>
- create_mip_view_with_content(&self, render_content: Arc<RenderContent>, pos: (i32, i32), size: (u32, u32)) -> Result<Box<dyn View>, Box<dyn Error>>

Example usage
```no_run
use std::sync::Arc;
use kepler_wgpu::rendering::{DefaultViewFactory, Orientation};
use kepler_wgpu::rendering::render_content::RenderContent;
use kepler_wgpu::data::ct_volume::CTVolume;

fn build_views(factory: &DefaultViewFactory, vol: &CTVolume) -> Result<(), Box<dyn std::error::Error>> {
    // Prebuild a shared RenderContent once
    // Note: callers can also build RenderContent directly via RenderContent::from_bytes/_r16f
    // depending on their data path.
    let bytes: Vec<u8> = /* prepare voxel bytes for texture */ vec![];
    let rc = RenderContent::from_bytes_r16f(
        &factory_device(),
        &factory_queue(),
        &bytes,
        "CT Volume",
        vol.dimensions.0 as u32,
        vol.dimensions.1 as u32,
        vol.dimensions.2 as u32,
    )?;
    let rc = Arc::new(rc);

    // Create MPR views with the same content
    let _mpr_transverse = factory.create_mpr_view_with_content(
        rc.clone(), vol, Orientation::Transverse, (0, 0), (512, 512)
    )?;
    let _mpr_coronal = factory.create_mpr_view_with_content(
        rc.clone(), vol, Orientation::Coronal, (520, 0), (512, 512)
    )?;

    // Create a MIP view reusing the same content
    let _mip_view = factory.create_mip_view_with_content(
        rc.clone(), (0, 520), (512, 512)
    )?;

    Ok(())
}
```

Performance considerations
- Reusing Arc<RenderContent> avoids repeated queue.write_texture and conversion costs (i16 → f16 or RG packing).
- This helps keep GPU operations non-blocking and reduces memory churn.
- For heavy TRACE logs in render loops, gate by the trace-logging feature and consider sampling to prevent flooding.

Cross-platform
- Native builds can control logging via RUST_LOG; wasm builds route logs to the browser console via console_log.
- The changes are compatible with both native and wasm targets.