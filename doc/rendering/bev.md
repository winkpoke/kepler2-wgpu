# Beam's-eye review core

`data::bev` owns validated f64 source-looking projection in patient LPS,
isocenter-plane mm, physical voxel-center mapping/native texture permutations,
fit bounds, patient directions, bounded original contour projection and ideal
leaf presentation rectangles. Beam +Z points toward source. No egui dependency.

`rendering::bev::BevRenderContext` borrows the host device and shared
`Arc<RenderContent>`. `prepare_target` permits output registration before encoding;
`target_generation` changes on replacement. `render` uses the host encoder. One
R32Float integral/RGBA8 display pair is retained, capped at 1024 px per edge.
Contrast reuses integration; geometry/detector/quality changes invalidate it.

The shader uses the shared signed decoder and explicit neighbor loads before
trilinear interpolation. It integrates `max(0,(HU+1000)/1000)` over the complete
CT intersection in mm. The 1..=4096 step cap adapts step length, never truncates
the ray. Display uses `1-exp(-k*integral)`, finite k in (0,0.2]. This is qualitative
DRR, not a calibrated kV/MV/portal/dose model.

`rendering::bev_overlay` owns the shared-pass aperture, jaw, translucent leaf
and original-contour GPU renderer. The bounded 106,000-item instance buffer is
lazy per context; scene and detector updates are independent. Attachment format
is explicit. Recreate on device/format change. No window/surface is owned.

The host wraps construction/allocation in error scopes, admits target/buffer
bytes, registers output views and releases them on source replacement. No
runtime, idle repaint, duplicate CT upload or second WGPU generation is added.
Association, machine setup/approval, selected beam/CP and GUI remain host policy.

Contour projection preserves original polyline edges/closure, with point limits
checked before construction. It does not reconstruct silhouettes or infer target
coverage. See the [approved change](../../openspec/changes/add-bev-review/proposal.md).

Checks: `cargo test --locked --lib data::bev` and native
`cargo test --locked --test bev_rendering_tests`. The latter uses the existing
fail-closed headless GPU helper. Native/WASM type checks and synthetic host UI
are separate from Windows/browser runtime and clinical acceptance.

## Contour capacity update — 2026-11-02

Filtered projection applies its point budget only to eligible ROI indices. The viewer
can request 500,000 source points without truncation. The overlay retains a hard
524,288-item bound (24 MiB at 48 bytes/item), including machine geometry. The GPU
instance buffer starts at 48 bytes and grows on demand, retaining its high-water
capacity until the context is dropped. Memory admission may conservatively reserve
the maximum. Source coordinates and quantitative data are unchanged.
