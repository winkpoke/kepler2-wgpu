# Design

`data::bev` owns f64 projection and physical/native voxel mapping, source-plane
rejection, original contour projection and leaf rectangles. APIs accept explicit
patient LPS and collimator basis inputs; no viewer/egui dependency enters Kepler.

`rendering::bev::BevRenderContext` borrows the host device/queue and retains its
shared `Arc<RenderContent>`. It owns shader/pipelines, two bounded offscreen
targets and cache keys. The host registers the color view with egui and rebuilds
registration when target generation changes. No volume upload is duplicated.

Integration samples the physical CT box along divergent rays, including beyond
isocenter. HU is converted to nonnegative relative water density and integrated
in mm. Display uses 1-exp(-k*integral); this is a qualitative projection, not an
energy/spectrum/calibrated attenuation model. Explicit neighbor loads use the
canonical signed decoder before trilinear interpolation. A bounded step budget
increases physical step size when needed, never truncating a ray.

Contrast changes rerender only the display target. Detector/beam/quality changes
invalidate integration. The context retains one current integral/color pair.
The host requests 512 px while interacting and 1024 px at rest, including target
allocation in its resource admission. No work is done before opening BEV or idle.

Original contour polylines are labelled as projected lines, not silhouettes.
Point limits reject a projection instead of silently truncating it. Rollback
removes the new modules and host adapter without changing existing APIs.
