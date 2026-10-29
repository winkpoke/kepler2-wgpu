# Design

Promote geometric aperture intersections and the MPR beam pipeline from the viewer
into data::beam and rendering::beam. A complete pane batch uses dynamic uniform
offsets and distinct aperture instance ranges. The context validates geometry and
clears stale ranges on rejection. Existing rendering::bev and bev_overlay own the
selected-beam projection, textures and overlays; the host retains egui adapters.

Source patient coordinates are rebased about the isocenter in f64 before GPU
conversion. Native/oblique slice planes and image clipping are supplied explicitly.
Selection changes style, not geometry or visibility. Delivery stepping does not
alter the plan arrangement. Existing single-beam pixel regressions remain applicable.

The exact axis-aligned recorded-aperture union also lives in data::beam. The host
only verifies a common recorded beam pose and forwards aperture outlines. The
union is bounded and distinct from the dashed envelope renderer; neither derives
fluence. Off-slice parallel-axis projections are explicitly dashed and host-labeled.
