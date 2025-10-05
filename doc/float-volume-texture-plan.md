# Floating-Point Volume Texture Path: Minimal, Non-Breaking Plan

Context
- Goal: Add a native floating-point volume texture path to improve precision and eliminate HU_OFFSET and byte packing/decoding, while preserving the current RG8 pipeline as default.
- Strategy (phased):
  - Phase 1: Implement R16Float (FP16) volume texture path first for broad filterability and reduced bandwidth.
  - Phase 2: Expand to R32Float support under an optional device feature gate for maximum precision.

Phased development plan

Phase 1: R16Float implementation (opt-in, non-breaking)
1) Capability check and opt-in runtime flag
- Change: Introduce a runtime flag `enable_float_volume_texture: bool` (default auto-enabled if supported) to enable the float path. At startup, if R16Float is supported for filtered sampling, select `wgpu::TextureFormat::R16Float`; otherwise default to `Rg8Unorm`.
- Files: src/state.rs
- Purpose: Keeps current RG8 path as default on unsupported devices; enables half-float textures automatically when supported.
- Stability: Behavior remains consistent on unsupported devices (falls back to RG8).

2) Volume texture creation for half-float data
- Change: Add a constructor/path to build a 3D texture using `wgpu::TextureFormat::R16Float`, alongside the existing RG8 path.
- Files: src/render_content.rs (alongside existing RG8 creation logic)
- Purpose: Provide native floating-point sampling without RG packing/decoding.
- Stability: Existing `from_bytes` remains unchanged; new half-float path is opt-in.
- Notes: R16Float is broadly filterable; allow linear filtering for better visualization quality.

3) Extend fragment uniforms to support both data types
- Change: Add a flag in fragment uniforms, e.g., `u_uniform_frag.is_packed_rg8`, indicating whether data is packed RG8 (decode) or native float (no decode).
- Files: src/view/render_context.rs (UniformsFrag)
- Purpose: Keeps shader logic generic for both formats; no bind layout upheaval.
- Stability: One extra uniform with a default value for current path (packed RG8) maintains existing behavior.

4) Shader branch for decoding based on the flag
- Change: In `src/shader/shader_tex.wgsl`, decode only when `is_packed_rg8` is true; otherwise treat the sampled value as native float (use `sample.r`), keeping window/level logic intact.
- Purpose: Eliminates HU_OFFSET and RG unpacking for the half-float path; preserves current RG8 behavior.
- Stability: Minimal change; uniform-controlled branch keeps output identical unless the float path is enabled.

5) Conditional upload path in `load_data_from_ct_volume`
- Change: When `enable_float_volume_texture` is true, convert voxels to half (FP16) on CPU and upload as R16Float with `bytes_per_row = 2 × columns` and `rows_per_image = rows`; otherwise continue with RG8 (`bytes_per_row = 2 × columns`).
- Files: src/state.rs (load_data_from_ct_volume)
- Purpose: Enables half-float texture path at upload time; leaves current behavior untouched otherwise.
- Stability: The stride matches RG8 in bytes-per-row, minimizing code changes. Use a well-tested half conversion (e.g., via a small dependency) to avoid precision surprises.

6) Keep swapchain/present and 2D pipeline unchanged
- Change: Do not modify the onscreen render target (remain `wgpu::TextureFormat::Rgba8Unorm`) and fragment color target formats.
- Files: src/view/render_context.rs
- Purpose: Ensures no change to current presentation pipeline and visuals by default.
- Stability: Guarantees identical output for the default path.

7) Validation and performance (Phase 1)
- Validation: With `enable_float_volume_texture = false`, verify visuals match current behavior. With it true, validate window/level matches expected results without unpacking or HU_OFFSET.
- Performance: R16Float halves bandwidth vs R32Float and is filterable across platforms.

Phase 2: R32Float expansion (optional, gated by device feature)
1) Device feature detection and selection policy
- Change: Detect the optional `float32-filterable` feature. When present and explicitly requested, prefer `wgpu::TextureFormat::R32Float` for maximum precision; otherwise fall back to R16Float (for linear filtering) or RG8.
- Files: src/state.rs
- Purpose: Use highest precision when supported without sacrificing broad compatibility.

2) Volume texture creation for 32-bit float
- Change: Add a constructor/path to build a 3D texture using `wgpu::TextureFormat::R32Float`.
- Files: src/render_content.rs
- Notes: If `float32-filterable` is absent, use nearest sampling; if present, enable linear filtering.

3) Upload path for 32-bit float
- Change: Upload voxels as f32 to the R32Float texture with `bytes_per_row = 4 × columns` and `rows_per_image = rows`.
- Files: src/state.rs
- Purpose: Provide uncompressed high-precision data with the simplest stride.

4) Validation and performance (Phase 2)
- Validation: Confirm identical window/level behavior without RG unpacking or HU_OFFSET. Compare against R16Float for expected fidelity improvements.
- Performance: R32Float doubles memory bandwidth vs R16Float; apply nearest sampling when filterability is not supported, or prefer R16Float for filtered sampling.

Implementation notes
- Reference: Current RG8 path unpacks two bytes in `shader_tex.wgsl` and uses `HU_OFFSET`. The float paths remove both.
- Strides:
  - RG8: `bytes_per_row = 2 × columns`.
  - Phase 1 (R16Float): `bytes_per_row = 2 × columns`.
  - Phase 2 (R32Float): `bytes_per_row = 4 × columns`.
- Dimensions: Respect CTVolume dimensions `(rows, columns, slices)` and voxel spacing `(spacing_x, spacing_y, spacing_z)` throughout.

Implemented changes and actions (current code)
- Capability and default selection
  - `enable_float_volume_texture` is initialized based on a runtime capability check; when the device supports filtered R16Float, the float path is selected by default.
  - A toggle function is available to switch formats at runtime; logs indicate the chosen default and any toggles.
- RenderContext defaults and logging
  - Defaults are format-aware: `window_level = 1140.0` for packed RG8, `window_level = 40.0` for float; `window_width = 350.0` for both.
  - `is_packed_rg8` is set to `1.0` (RG8) or `0.0` (float). Initialization logs: "RenderContext defaults => window_width, window_level, is_packed_rg8".
- Shader decoding branch
  - Fragment shader decodes RG8 using `(g * 256 + r) * 255.0` only when `is_packed_rg8 > 0.5`; otherwise uses `sampled_value.r` directly for the float path. Window/level logic is shared.
- Upload paths
  - Float path: Convert voxels to half (`f16`) on CPU and upload to `R16Float` with `bytes_per_row = 2 × columns`, `rows_per_image = rows`.
  - RG8 path: Add `HU_OFFSET` on CPU, pack to two bytes per voxel, and upload with `bytes_per_row = 2 × columns`.
- Uniform updates each frame
  - `GenericMPRView::update` writes vertex and fragment uniforms every frame using the queue, ensuring window/level and slice changes are applied immediately.
- Texture and sampling
  - 3D textures are created with `TextureDimension::D3` and sampled via `texture_3d<f32>`; sampler uses filtering consistent with device support.
- Event injections for validation
  - `main.rs` currently has slice/scale/translate injections commented out (e.g., `set_slice_mm(0, 5.0)`), retained for manual testing if needed.
- Known visual issue and guidance
  - If the transverse view shows black, verify slice depth and pan are in-bounds; out-of-bounds coordinates are clamped to black by the shader. Adjust via `set_slice_mm` and pan controls.

Change tracking (this document)
- Modified: Clarified the default selection policy for `enable_float_volume_texture` (auto-enabled when supported; otherwise RG8).
- Modified: Corrected RenderContext default `window_width` to 350.0 and documented format-aware `window_level` defaults (1140.0 for RG8, 40.0 for float).
- Added: Implemented changes and actions section summarizing capability detection, logging, shader branching, upload paths, uniform updates, and event injection status.