# Floating-Point Volume Texture Path: Minimal, Non-Breaking Plan

Context
- Goal: Add a native floating-point volume texture path to improve precision and eliminate HU_OFFSET and byte packing/decoding, while preserving the current RG8 pipeline as default.
- Strategy (phased):
  - Phase 1: Implement R16Float (FP16) volume texture path first for broad filterability and reduced bandwidth.
  - Phase 2: Expand to R32Float support under an optional device feature gate for maximum precision.

Phased development plan

Phase 1: R16Float implementation (opt-in, non-breaking)
1) Capability check and opt-in runtime flag
- Change: Introduce a runtime flag `enable_float_volume_texture: bool` (default false) to enable the float path. For Phase 1, select `wgpu::TextureFormat::R16Float` when enabled.
- Files: src/state.rs
- Purpose: Keeps current RG8 path as default; enables half-float textures explicitly.
- Stability: No behavior changes by default; safe for WASM/older GPUs.

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

---

Comparison: R32Float vs R16Float (Precision, Performance, and Filtering)
- Precision and dynamic range:
  - R32Float: Highest precision and dynamic range; ideal when clinical HU fidelity, strict window/level operations, and avoidance of quantization artifacts are paramount.
  - R16Float: Half-precision format; adequate for many visualization tasks but with reduced precision and range. Best when storage/bandwidth constraints and compatibility are more important than maximum fidelity.
- Memory bandwidth and footprint:
  - R32Float: 4 bytes per voxel; doubles bandwidth and memory use versus R16Float.
  - R16Float: 2 bytes per voxel; lower bandwidth and memory, improving throughput on large volumes.
- Which offers superior quality:
  - Prefer R32Float for the most faithful CT/HU representation and robust window/level behavior.
  - Prefer R16Float when acceptable quality suffices and you need better performance and broader platform support.

Linear filtering support and considerations
- R16Float: Supported for linear filtering broadly; suitable for interpolated sampling without special device features.
- R32Float: Linear filtering is optional and requires enabling the device’s `float32-filterable` feature; many desktop GPUs support it, but mobile/web targets may not. When unavailable, use nearest sampling or fall back to R16Float for linear filtering.

Practical guidance
- If you need ubiquitous linear filtering and lower bandwidth, choose R16Float.
- If maximum precision is key and your target devices support `float32-filterable`, choose R32Float; otherwise use nearest sampling or fall back to R16Float.

Device feature checks
- At runtime, query adapter/device features for `float32-filterable` to decide whether R32Float can be filtered. If not supported, keep R32Float with nearest sampling or select R16Float for filtered sampling.

Change tracking
- Updated strategy to phased approach (Phase 1: R16Float; Phase 2: R32Float).
- Adjusted stride notes to differentiate half vs 32-bit paths.
- Clarified filtering behavior and device feature gating for R32Float.
- Documented upload details: R16Float uses 2 × columns; R32Float uses 4 × columns.