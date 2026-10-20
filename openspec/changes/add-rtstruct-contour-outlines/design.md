# Design

Data retains f64 patient LPS points, contour kind, ROI identity/color and image references independently of graphics. The importer uses the current DICOM decoder with limits of 32 MiB input, 4096 ROIs, 50000 contours and 500000 points. It rejects invalid cardinality, finite/planar geometry, duplicate/undefined ROI numbers and mixed XOR semantics. CT identity remains separate from display reindexing; existing CTImage constructors are unchanged.

The host matches ROI Frame of Reference, referenced series and explicitly referenced images. Missing CT identity or geometry prevents automatic attachment. With no supplied series/image references, matching the ROI frame is sufficient and is labelled as frame-only association. The viewer retains its current CT series selection; association failure requests the correct CT rather than silently drawing or registering.

The renderer uses the host device, queue, explicit attachment format and single-sample pass. Cached immutable instanced triangle segments are rebased in f64 before f32 conversion; uniforms are per pane. Alpha-blended, constant-screen-width round strokes have analytic antialiasing and no depth attachment. Exact source-plane outlines require parallel normals and points within 0.01 mm, not a slice-thickness slab. Point, open planar, closed and XOR outlines are displayed; open nonplanar curves are retained with a visible limitation. Fill and derived MPR/mesh representation are deferred. The host supplies the same patient/UV/clipping mapping as the image.

The viewer schedules worker I/O after first paint and uses generation checks, bounded buffers, event-driven repaint and source replacement invalidation. GPU allocation errors are captured by the host. No egui/window/runtime ownership enters the new Kepler modules. Existing signed-volume encoding, shaders and palette remain unchanged.

Alternative: egui polylines are adequate for a tiny prototype but retessellate at repaint. A single integer label map loses overlaps. Rollback removes additive host integration/modules without migrations.
