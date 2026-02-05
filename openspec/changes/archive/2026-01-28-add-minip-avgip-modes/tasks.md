# Tasks

- [x] Update `MipUniforms` struct in `src/rendering/view/mip/mod.rs` to include `mode` and adjust padding.
- [x] Update `MipUniforms` struct in `src/rendering/shaders/mip.wgsl` to include `mode`.
- [x] Implement MinIP and AvgIP logic in `src/rendering/shaders/mip.wgsl`.
- [x] Update `MipViewWgpuImpl::update_uniforms` in `src/rendering/view/mip/mod.rs` to pass the mode to the GPU.
- [x] Verify MinIP rendering works.
- [x] Verify AvgIP rendering works.
