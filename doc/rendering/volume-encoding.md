# Signed voxel GPU encoding

Canonical CPU voxels remain `i16`; DICOM rescaling happens before GPU encoding.
`src/data/volume_encoding.rs` owns byte encoding, encoding-to-texture-format
mapping, and storage decode parameters. `Graphics` owns a capability snapshot for
its selected device, and `App` uses that snapshot to choose a usable representation.

## Representations

- `VolumeEncoding::HuFloat`: preferred when filtered sampling is supported.
  Converts voxels to IEEE half-float; large integers can round (32767 becomes
  32768). This is a display representation, not lossless integer storage.
- `VolumeEncoding::for_float_support(false)`: exact compatibility storage in `Rg8Unorm`.
  Encode `u16(i32(voxel) + 32768)` as little-endian bytes, R low and G high.
  Every `i16` value is representable, with no clamping or overflow.

The 32768 offset is a property of the signed integer domain, not a CT lower
bound. It must never be applied to window/level settings. Shader uniforms keep
storage decode parameters separate from display parameters.

## Device capabilities and upload limits

A format is eligible for volume rendering only when the selected device permits
`TEXTURE_BINDING | COPY_DST` and filtered sampling (`FILTERABLE`) under the
features enabled on that device. `R16Float` is preferred when eligible; otherwise
`HuPackedRg8 { offset: 32768.0 }` is used for canonical signed uploads. Backend identity alone does not determine encoding. Device
replacement reevaluates eligibility while retaining the prior preference when the
new device supports it; it also retains CPU view interaction state.

`RenderContent::from_bytes` checks that each dimension is nonzero and no greater
than `device.limits().max_texture_dimension_3d`, and that the input has exactly
two bytes per voxel with checked size arithmetic. Invalid dimensions or byte
counts return an error before GPU texture allocation. Data is never clamped,
resampled, or truncated to fit a device. The maximum is per axis, not a total
voxel-count limit.

## Filtering and decoding

MPR, MIP, and DVR prepend `volume_decode.wgsl` via `volume_shader_source` and use the
same affine decoder. Byte channels are not rounded before decoding: doing so
would destroy interpolation.

Production volume textures have one mip level. The release R16Float sampler uses
linear minification and magnification; packed RG8 uses nearest minification and
linear magnification. Both policies are retained. A fixed LOD zero is **not** an equivalent
replacement for implicit sampling because it can force magnification filtering
when the image is downsampled. MPR samples before per-fragment bounds returns to
keep implicit derivatives in uniform control flow, discarding that sample for
black-background pixels. MIP computes the orthographic ray-start derivatives
before miss returns/ray loops and derives `log2(max(length(dx), length(dy)))` in
texel units (with a small zero-footprint floor). Its helper uses that explicit LOD,
retaining min/mag selection without implicit derivatives in divergent control.
Ray direction and step size are uniform for MIP, so advancing by a logical ray step does
not change its footprint. DVR derives an entry-point footprint before divergent
ray traversal and uses explicit LOD for volume and normal samples. Neither signed decoding nor projection aggregation,
thresholds, bounds values, or window/level is changed.

Raw two's-complement bytes were deliberately not chosen: interpolating encoded
values across the sign boundary does not interpolate the signed voxels. Such
an encoding would require decoding each neighbor before manual interpolation,
or limiting packed textures to nearest sampling.

RG8 bytes preserve integer data exactly. Normalized GPU sampling and arithmetic
can introduce small floating-point error; discrete samples recover the exact
integer by rounding. The GPU regression checks every integer plus a <0.01
absolute float-error bound, and verifies linear filtering across zero and a
low-byte carry.

The public release variants `HuPackedRg8 { offset: f32 }` and `HuFloat` remain
unchanged. Caller-supplied bytes retain their specified decode offset, including
the release default 1100. Canonical CT uploads use offset 32768; callers do not
need to regenerate legacy buffers to keep using their declared representation.
Production loaders use runtime data; retired opaque compile-time assets remain
retired. Tests use deterministic synthetic volumes.

## Verification

```bash
cargo test --test volume_encoding_tests
# Require an adapter, rather than allowing GPU-less CI to skip the GPU portion:
KEPLER_REQUIRE_GPU_TESTS=1 cargo test --test volume_encoding_tests
```

Coverage includes exhaustive CPU and application-upload roundtrips, endpoint
and CT examples, byte ordering, empty/missing data, unchanged half-float
rounding, production WGSL validation, GPU upload/readback, and GPU filtering.

The release default offset remains 1100. Caller-provided packed bytes retain their supplied offset; only canonical signed CT uploads use 32768. R8 scalar and R8Uint label APIs retain their release metadata and sampler policies.
