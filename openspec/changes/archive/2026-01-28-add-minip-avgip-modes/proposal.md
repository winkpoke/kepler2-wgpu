# Add MinIP and AvgIP Rendering Modes

## Why
Medical imaging often requires different projection modes to visualize different structures:
- **MIP (Maximum Intensity Projection)**: Visualizes high-density structures (bones, contrast-filled vessels).
- **MinIP (Minimum Intensity Projection)**: Visualizes low-density structures (airways, lungs).
- **AvgIP (Average Intensity Projection)**: Visualizes internal structures with depth cues, similar to a traditional X-ray.

## What Changes
- Add MinIP and AvgIP modes to the existing MIP pipeline via a single `mode` uniform.
- Update `mip.wgsl` ray-march accumulation for MIP/MinIP/AvgIP without duplicating pipelines.
- Extend Rust/WGSL `MipUniforms` with `mode` while keeping 16-byte alignment.

## Alternatives Considered
- **Separate Shaders**: Rejected to avoid code duplication and simplify pipeline management. The branching overhead in the loop is acceptable for this MVP use case.
