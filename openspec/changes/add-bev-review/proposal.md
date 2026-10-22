# BEV review core

The owner approved the BEV + MPR design and implementation on 2026-10-21,
then explicitly directed that algorithms and rendering live in Kepler while
the egui host retains superficial GUI. This authorizes the sibling library work.

Add source-looking isocenter projection, validated physical/native texture
mapping, bounded original contour projection, ideal leaf display geometry and
a lazy cached GPU qualitative divergent CT projection. Reuse RenderContent,
signed volume decode and the host WGPU device. No dependency changes.

The host owns plan/CT associations, selected beam/recorded CP, labels, controls,
egui texture registration and UI navigation. Do not infer machine interpolation,
clinical approval, reconstructed ROI silhouettes or commissioned kV/MV imaging.
Existing RTPLAN model ownership migration remains a separate change.

Acceptance: analytical synthetic geometry and GPU slabs, cache/resource tests,
native viewer UI, native and WASM types, relevant lint/format/docs/spec checks,
and measured executable/startup effects. Platform limits remain explicit.
