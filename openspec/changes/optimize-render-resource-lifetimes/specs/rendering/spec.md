# Rendering Specification Delta

## ADDED Requirements

### Requirement: Persistent Mesh Depth Attachments

The rendering subsystem SHALL retain at most one reusable mesh depth texture and its view within the owning graphics context. Reuse SHALL be keyed by width, height, sample count, and depth format, and resources SHALL NOT be reused across devices.

#### Scenario: Repeated compatible mesh frames
- **WHEN** successive mesh frames use the same device and depth attachment key
- **THEN** the renderer SHALL reuse the same depth texture and cached depth view
- **AND** it SHALL NOT allocate an offscreen color texture for a mesh pass rendering directly to the surface

#### Scenario: Depth attachment key changes
- **WHEN** width, height, sample count, or depth format changes
- **THEN** the next mesh frame SHALL replace the cached attachment with a compatible texture and view
- **AND** obsolete cache entries SHALL NOT accumulate

#### Scenario: Zero-sized surface
- **WHEN** either requested surface dimension is zero
- **THEN** no depth texture SHALL be allocated for that request
- **AND** no render pass SHALL use a zero-sized attachment

#### Scenario: Graphics replacement or removal
- **WHEN** the owning graphics context is replaced or destroyed
- **THEN** its cached depth resources SHALL be discarded
- **AND** a replacement context SHALL NOT reuse attachments from the old device

#### Scenario: Mesh views removed
- **WHEN** the layout no longer contains mesh views
- **THEN** depth resource retention SHALL remain bounded to one cache entry or the entry SHALL be released
- **AND** no additional depth resources SHALL be allocated until mesh rendering resumes

### Requirement: Shared Immutable MPR Resources

MPR views created or bulk-rebound through the same device/attachment-format factory configuration SHALL share one immutable `MprRenderContext`, including its pipeline, bind-group layouts, and quad geometry buffers. View-specific uniform buffers and bind groups SHALL remain independent. Views sharing volume content SHALL reuse that content's sampler.

#### Scenario: Multiple anatomical views
- **WHEN** axial, coronal, sagittal, or oblique views are created by the same factory configuration
- **THEN** they SHALL reference the same immutable MPR context
- **AND** updating one view's uniforms SHALL NOT alter another view's uniforms

#### Scenario: Ordinary resize
- **WHEN** view or surface dimensions change without a device or attachment-format change
- **THEN** the shared MPR pipeline resources SHALL remain valid and SHALL NOT be rebuilt solely for the resize

#### Scenario: Bulk GPU rebinding
- **WHEN** existing MPR views are rebound after a device or attachment-format change
- **THEN** all such views SHALL share a compatible replacement context
- **AND** their slice/orientation, window/level, pan, and zoom state SHALL be preserved
- **AND** resources from an incompatible device or attachment format SHALL NOT be used

### Requirement: Complete Frames With Dirty Uniform Updates

The rendering subsystem SHALL avoid recomputing or uploading unchanged per-view uniform state, but SHALL draw every visible view required for a complete image whenever a new surface frame is acquired. It SHALL NOT rely on a surface image preserving pixels from a previous presentation.

#### Scenario: One view changes in a multi-view layout
- **WHEN** one view is modified and a new surface frame is acquired
- **THEN** the modified view's pending GPU state SHALL be uploaded
- **AND** unchanged views SHALL NOT receive redundant uniform uploads
- **AND** all visible views SHALL be drawn so their content remains present in the resulting image

#### Scenario: GPU resources rebuilt
- **WHEN** a view's GPU resources are replaced
- **THEN** required uniforms SHALL be uploaded before rendering with those resources
- **AND** the view SHALL remain pending presentation until a frame succeeds

#### Scenario: Failed frame
- **WHEN** acquisition or frame rendering fails before successful submission and presentation
- **THEN** pending visual invalidation SHALL NOT be discarded
- **AND** a subsequent successful frame SHALL include the pending changes
