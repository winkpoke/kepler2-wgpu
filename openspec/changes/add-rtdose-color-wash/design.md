# Design

DoseGrid retains original u16/u32 samples and f64 scaling/geometry. DoseGeometry retains orthonormal in-plane directions, spacing, and strictly ascending physical frame offsets after canonicalizing descending inputs together with pixel frames. Import rejects unsupported semantics explicitly.

DoseTexture owns one R32Float approximate display copy and a frame-offset storage buffer. DoseView derives a rebased physical slice-to-grid matrix in f64 before f32 uniforms. The shader uses explicit neighbor loads, physical Z bracketing and interpolation, then a fixed color ramp and premultiplied-alpha blending. No float32 filtering feature, CT resampling, second WGPU device or CT encoding change is required. Host allocation/validation scopes catch creation errors; uniforms are independent per pane.

SlicePlane is shared with contours; the public StructurePlane alias remains compatible. Viewer dose loading uses a bounded header probe followed by one bounded object parse on the worker. Metadata may be inspected without CT; overlay requires valid CT geometry and the same Frame of Reference. New CT clears dose and stale worker results cannot attach. Display admission counts encoded CT, dose texture and dose upload staging under the existing 2 GiB policy; file parsing/original samples and driver memory are separately bounded, not claimed as total RSS.

Rollback removes the new dose modules and viewer adapter; existing CT and contour interfaces remain usable.
