# Change: Add MIP Spin Control

## Why
Users need to view the MIP volume from different angles to better understand 3D structure. The current MIP view is fixed to an orthographic +Z ray direction, so the only way to change the view is pan/zoom, which is insufficient for many clinical review tasks.

Current codebase gaps that make “spin” non-trivial and need to be addressed explicitly in the proposal:
- There is no application-level interaction pathway for MIP rotation (no user event / AppView API).
- The MIP shader assumes axis-aligned rays and hardcodes the volume intersection interval, which will be incorrect once rotation changes the ray direction.
- MIP uniform layout is currently sized tightly (64 bytes) and validated by tests; adding rotation parameters must preserve correct WGSL/Rust layout and alignment.

## What Changes
- Add MIP rotation state and a public, type-safe interface to update it (roll/yaw/pitch).
- Extend the MIP uniform buffer to carry rotation parameters in a GPU-friendly format.
- Update the MIP shader to apply the rotation and to compute correct ray/volume intersection for rotated rays.
- Add a minimal application-layer route for rotation control that matches existing interaction patterns (App → AppView → specific view).
- Add validation coverage for the new uniform layout and for “rotation changes output” smoke behavior.

Explicit constraints to keep scope feasible:
- This change targets manual rotation via explicit angles (not continuous animation).
- Existing MIP settings that are currently no-ops in shader (e.g., mode/slab) remain out of scope for this change, but are documented as follow-ups.

## Impact
- Affected specs: `rendering`, `application`
- Affected code:
  - `src/rendering/view/mip/mod.rs`
  - `src/rendering/shaders/mip.wgsl`
  - Application interaction routing (App/AppView/UserEvent) and any web control surface that exposes view parameters
