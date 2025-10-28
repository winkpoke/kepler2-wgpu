# WGPU 27 and Winit 0.30 Upgrade Plan

**Date**: 2025-10-28T07:54:10  
**Status**: Planning Phase  
**Priority**: High  
**Current Versions**: wgpu 23.0.0, winit 0.29.15  
**Target Versions**: wgpu 27.0.1, winit 0.30.x  
**Estimated Timeline**: 7-10 days  

## Executive Summary

This document outlines the comprehensive upgrade plan for the Kepler WGPU medical imaging framework to migrate from wgpu 23.0.0 to 27.0.1 and winit 0.29.15 to 0.30.x. The upgrade involves significant breaking changes, particularly around raw-window-handle compatibility and API modernization.

## Current State Analysis

### Current Dependencies
```toml
# Native dependencies
winit = { version = "0.29.15", features = ["rwh_05"] }
wgpu = "23.0.0"

# WASM dependencies  
wgpu = { version = "23.0", features = ["webgl"]}
```

### Key Compatibility Issues
- **Raw Window Handle**: Currently using `rwh_05` feature, need to upgrade to `rwh_06` <mcreference link="https://www.reddit.com/r/rust/comments/18ceig0/how_did_i_need_to_know_about_feature_rwh_05_for/" index="1">1</mcreference>
- **API Changes**: Multiple breaking changes across wgpu versions 24-27 <mcreference link="https://github.com/gfx-rs/wgpu/releases" index="1">1</mcreference>
- **MSRV Requirements**: wgpu 27 requires Rust 1.82+ for core components and 1.88+ for workspace <mcreference link="https://docs.rs/crate/wgpu/latest" index="2">2</mcreference>

## Breaking Changes Analysis

### WGPU 23 → 27 Major Changes

#### Version 24.0 Changes
- **Device Creation API**: `requestDevice` now takes one parameter instead of 2 <mcreference link="https://sotrh.github.io/learn-wgpu/news/25.0/" index="4">4</mcreference>
- **Trace Configuration**: Moved into `DeviceDescriptor` <mcreference link="https://sotrh.github.io/learn-wgpu/news/25.0/" index="4">4</mcreference>

#### Version 25.0+ Changes  
- **Surface Creation**: Introduction of `wgpu::SurfaceTarget` for safe surface creation <mcreference link="https://github.com/gfx-rs/wgpu/blob/trunk/CHANGELOG.md" index="3">3</mcreference>
- **Raw Window Handle**: Automatic conversion for types implementing `HasWindowHandle` & `HasDisplayHandle` traits <mcreference link="https://github.com/gfx-rs/wgpu/blob/trunk/CHANGELOG.md" index="3">3</mcreference>

#### Version 26.0+ Changes
- **Features Restructuring**: Internal split of Features struct with new namespaces (`FeaturesWGPU`, `FeaturesWebGPU`) <mcreference link="https://github.com/gfx-rs/wgpu/blob/trunk/CHANGELOG.md" index="3">3</mcreference>
- **Memory Hints**: New `memory_hints` field in `DeviceDescriptor` <mcreference link="https://sotrh.github.io/learn-wgpu/news/25.0/" index="4">4</mcreference>

#### Version 27.0+ Changes
- **HAL Changes**: Breaking change in `wgpu_hal::vulkan::Device::texture_from_raw` with new `&self` receiver <mcreference link="https://github.com/gfx-rs/wgpu/releases" index="1">1</mcreference>
- **Buffer API**: `BufferSlice::get_mapped_range_as_array_buffer()` changed to `BufferView::as_uint8array()` <mcreference link="https://github.com/gfx-rs/wgpu/releases" index="1">1</mcreference>

### Winit 0.29 → 0.30 Major Changes

#### Event Loop API Modernization
- **Trait-Based API**: Moving towards trait-based API alongside closure-based API <mcreference link="https://github.com/rust-windowing/winit/issues/3476" index="4">4</mcreference>
- **Migration Period**: v0.30 serves as migration step with both APIs available <mcreference link="https://github.com/rust-windowing/winit/issues/3476" index="4">4</mcreference>

#### Raw Window Handle Upgrade
- **RWH 0.6**: Upgrade from raw-window-handle 0.5 to 0.6 <mcreference link="https://github.com/neovide/neovide/pull/2698" index="5">5</mcreference>
- **Compatibility**: Requires coordinated upgrade with wgpu for RWH compatibility

#### Wayland Backend Rewrite
- **wayland-rs 0.30**: Complete rewrite using new wayland-rs 0.30 API <mcreference link="https://github.com/rust-windowing/winit/issues/2128" index="3">3</mcreference>
- **Forward Compatibility**: Fixes long-standing forward compatibility issues <mcreference link="https://github.com/rust-windowing/winit/issues/2128" index="3">3</mcreference>

## Upgrade Strategy

### Phase 1: Pre-Upgrade Preparation (Days 1-2)

#### 1.1 Environment Setup
```bash
# Verify Rust version compatibility
rustc --version  # Must be >= 1.88 for full compatibility

# Create backup branch
git checkout -b backup/pre-wgpu27-upgrade
git push origin backup/pre-wgpu27-upgrade

# Create upgrade branch
git checkout -b feature/wgpu27-winit030-upgrade
```

#### 1.2 Dependency Analysis
- **Audit Current Usage**: Identify all wgpu and winit API usage in codebase
- **Test Coverage**: Ensure comprehensive test coverage before upgrade
- **Documentation**: Document current behavior for regression testing

#### 1.3 Compatibility Research
- **Third-Party Crates**: Verify all dependencies support new versions
- **Feature Flags**: Review and update feature flag requirements
- **Platform Testing**: Prepare testing strategy for Windows, macOS, Linux, and WASM

### Phase 2: Core Dependency Upgrade (Days 3-4)

#### 2.1 Cargo.toml Updates

**Native Dependencies**:
```toml
[dependencies]
winit = { version = "0.30", features = ["rwh_06"] }  # Updated from rwh_05
wgpu = "27.0.1"  # Major version upgrade
pollster = "0.4.0"  # Verify compatibility
env_logger = "0.11.5"  # Should remain compatible
```

**WASM Dependencies**:
```toml
[target.'cfg(target_arch = "wasm32")'.dependencies]
wgpu = { version = "27.0.1", features = ["webgl"]}  # Updated version
winit = { version = "0.30", features = ["rwh_06"] }  # Consistent with native
```

#### 2.2 Raw Window Handle Migration
- **Feature Flag**: Change from `rwh_05` to `rwh_06`
- **API Updates**: Update surface creation code to use new `SurfaceTarget` API
- **Trait Implementation**: Ensure window types implement required traits

### Phase 3: API Migration (Days 5-6)

#### 3.1 Device Creation Updates
**Current Code Pattern**:
```rust
let (device, queue) = adapter.request_device(
    &wgpu::DeviceDescriptor {
        required_features: wgpu::Features::empty(),
        required_limits: wgpu::Limits::default(),
        label: None,
    },
    None, // trace
).await?;
```

**Updated Code Pattern**:
```rust
let (device, queue) = adapter.request_device(
    &wgpu::DeviceDescriptor {
        required_features: wgpu::Features::empty(),
        required_limits: wgpu::Limits::default(),
        label: None,
        memory_hints: Default::default(),  // New field
        trace: wgpu::Trace::Off,  // Moved into descriptor
    }
).await?;
```

#### 3.2 Surface Creation Updates
**Current Code Pattern**:
```rust
let surface = instance.create_surface(&window)?;
```

**Updated Code Pattern**:
```rust
let surface = instance.create_surface(wgpu::SurfaceTarget::Window(Box::new(&window)))?;
// Or for automatic conversion:
let surface = instance.create_surface(&window)?;  // Should still work with HasWindowHandle
```

#### 3.3 Features API Updates
- **Namespace Usage**: Update to use new feature namespaces if needed
- **WebGPU Compatibility**: Use `FeaturesWebGPU` for web-compatible features
- **WGPU Extensions**: Use `FeaturesWGPU` for native-only features

### Phase 4: Medical Imaging Specific Updates (Days 7-8)

#### 4.1 Texture and Buffer Management
- **Buffer Mapping**: Update any usage of `get_mapped_range_as_array_buffer()` to `as_uint8array()`
- **Texture Creation**: Review texture creation for any HAL-level changes
- **Memory Management**: Verify memory hints configuration for medical imaging workloads

#### 4.2 Rendering Pipeline Updates
- **Shader Compatibility**: Verify WGSL shader compatibility with new version
- **Compute Shaders**: Test CT reconstruction and MPR compute shaders
- **Performance Validation**: Benchmark rendering performance post-upgrade

#### 4.3 Multi-Platform Validation
- **Native Builds**: Test on Windows, macOS, Linux
- **WASM Builds**: Verify WebGL and WebGPU compatibility
- **Medical Accuracy**: Validate CT reconstruction accuracy

### Phase 5: Testing and Validation (Days 9-10)

#### 5.1 Automated Testing
```bash
# Native testing
cargo test --all-features
cargo build --release

# WASM testing  
wasm-pack build -t web
# Manual browser testing required (no npx/live-server per project rules)
```

#### 5.2 Integration Testing
- **Window Resizing**: Test MPR view aspect ratio preservation
- **Multi-View Layout**: Validate grid layout functionality
- **Medical Imaging**: Test CT loading, reconstruction, and visualization
- **Performance**: Benchmark against pre-upgrade performance

#### 5.3 Regression Testing
- **Visual Validation**: Compare rendering output with baseline images
- **Functionality**: Verify all medical imaging features work correctly
- **Error Handling**: Test error conditions and recovery

## Risk Assessment and Mitigation

### High-Risk Areas

#### 1. Raw Window Handle Compatibility
**Risk**: Incompatible RWH versions causing compilation failures
**Mitigation**: 
- Gradual migration with feature flags
- Comprehensive testing on all target platforms
- Fallback plan to intermediate versions if needed

#### 2. Surface Creation Changes
**Risk**: Breaking changes in surface creation affecting window management
**Mitigation**:
- Thorough testing of window creation and resizing
- Platform-specific validation
- Maintain compatibility layer if needed

#### 3. Medical Imaging Accuracy
**Risk**: Rendering changes affecting diagnostic accuracy
**Mitigation**:
- Pixel-perfect comparison testing
- Medical imaging validation with known datasets
- Performance regression testing

### Medium-Risk Areas

#### 1. WASM Compatibility
**Risk**: WebGL/WebGPU feature changes affecting browser support
**Mitigation**:
- Test on multiple browsers
- Maintain WebGL fallback
- Progressive enhancement approach

#### 2. Performance Regression
**Risk**: New version introducing performance overhead
**Mitigation**:
- Comprehensive benchmarking
- Memory usage monitoring
- GPU performance validation

## Rollback Strategy

### Immediate Rollback
```bash
# Quick rollback to working state
git checkout backup/pre-wgpu27-upgrade
git checkout -b hotfix/rollback-upgrade
```

### Partial Rollback Options
- **Version Pinning**: Pin to intermediate versions (e.g., wgpu 25.x, winit 0.29.x)
- **Feature Flags**: Disable problematic features temporarily
- **Platform-Specific**: Rollback only on problematic platforms

## Success Criteria

### Functional Requirements
- [ ] All existing medical imaging functionality preserved
- [ ] Native builds work on Windows, macOS, Linux
- [ ] WASM builds work in modern browsers
- [ ] Window resizing and layout management functional
- [ ] CT reconstruction accuracy maintained

### Performance Requirements
- [ ] No significant performance regression (< 5%)
- [ ] Memory usage remains within acceptable bounds
- [ ] GPU utilization efficiency maintained
- [ ] Startup time not significantly impacted

### Quality Requirements
- [ ] All existing tests pass
- [ ] New version-specific tests added
- [ ] Code review completed
- [ ] Documentation updated

## Timeline and Milestones

### Week 1: Preparation and Core Upgrade
- **Days 1-2**: Phase 1 (Preparation)
- **Days 3-4**: Phase 2 (Core Dependencies)
- **Day 5**: Initial compilation and basic testing

### Week 2: Implementation and Validation
- **Days 6-7**: Phase 3 (API Migration)
- **Days 8-9**: Phase 4 (Medical Imaging Updates)
- **Day 10**: Phase 5 (Testing and Validation)

### Week 3: Finalization (if needed)
- **Days 1-2**: Bug fixes and optimization
- **Days 3-4**: Final testing and documentation
- **Day 5**: Deployment preparation

## Post-Upgrade Tasks

### Documentation Updates
- [ ] Update `doc/CHANGELOG.md` with upgrade details
- [ ] Update build instructions in README
- [ ] Document any API changes affecting users
- [ ] Update dependency documentation

### Monitoring and Validation
- [ ] Monitor performance metrics post-deployment
- [ ] Collect user feedback on stability
- [ ] Validate medical imaging accuracy in production
- [ ] Plan future upgrade cycles

## Conclusion

This upgrade plan provides a comprehensive approach to migrating the Kepler WGPU medical imaging framework to the latest versions of wgpu and winit. The phased approach minimizes risk while ensuring all medical imaging functionality is preserved and enhanced.

The upgrade will provide access to the latest GPU features, improved performance, better platform compatibility, and enhanced developer experience while maintaining the high standards required for medical imaging applications.

**Key Success Factors**:
- Thorough testing at each phase
- Maintaining medical imaging accuracy
- Comprehensive rollback planning
- Platform-specific validation
- Performance monitoring throughout the process