# Change: Add 3D Volume Rendering in Mesh View

## Why
The current 3D view focuses on mesh visualization and does not provide volumetric rendering comparable to clinical tools like 3D Slicer, limiting diagnostic context for soft tissue and intensity-based inspection.

## What Changes
- Introduce GPU volume rendering for the 3D mesh view using ray-marched volume sampling
- Add a minimal transfer function for opacity/intensity control suitable for CT volumes
- Keep orthographic projection defaults to preserve medical measurement fidelity

## Impact
- Affected specs: rendering
- Affected code: rendering/view/mesh, rendering/core, shaders
