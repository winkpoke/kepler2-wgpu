# Signed voxel GPU encoding

Canonical CPU voxels remain `i16`; DICOM rescaling happens before GPU encoding.
`src/data/volume_encoding.rs` owns byte encoding, texture format selection, and
storage decode parameters. Both `AppModel` and `DefaultViewFactory` use it.

## Representations

- `VolumeEncoding::R16Float`: preferred when filtered sampling is supported.
  Converts voxels to IEEE half-float; large integers can round (32767 becomes
  32768). This is a display representation, not lossless integer storage.
- `VolumeEncoding::PackedI16Rg8`: exact compatibility storage in `Rg8Unorm`.
  Encode `u16(i32(voxel) + 32768)` as little-endian bytes, R low and G high.
  Every `i16` value is representable, with no clamping or overflow.

The 32768 offset is a property of the signed integer domain, not a CT lower
bound. It must never be applied to window/level settings. Shader uniforms keep
storage decode parameters separate from display parameters.

## Filtering and decoding

MPR and MIP prepend `volume_decode.wgsl` via `volume_shader_source` and use the
same affine decoder. Byte channels are not rounded before decoding: doing so
would destroy interpolation.

Raw two's-complement bytes were deliberately not chosen: interpolating encoded
values across the sign boundary does not interpolate the signed voxels. Such
an encoding would require decoding each neighbor before manual interpolation,
or limiting packed textures to nearest sampling.

RG8 bytes preserve integer data exactly. Normalized GPU sampling and arithmetic
can introduce small floating-point error; discrete samples recover the exact
integer by rounding. The GPU regression checks every integer plus a <0.01
absolute float-error bound, and verifies linear filtering across zero and a
low-byte carry.

The compile-time fixture loader encodes the original signed
`image/combined_pixel_array.bin`, rather than assuming the old prepacked binary
uses the current storage representation. Existing prepacked RG8 buffers must
be regenerated; the old `HuPackedRg8 { offset }` and `HuFloat` enum variants are
replaced by the two representations above.

## Verification

```bash
cargo test --test volume_encoding_tests
# Require an adapter, rather than allowing GPU-less CI to skip the GPU portion:
KEPLER_REQUIRE_GPU_TESTS=1 cargo test --test volume_encoding_tests
```

Coverage includes exhaustive CPU and application-upload roundtrips, endpoint
and CT examples, byte ordering, empty/missing data, unchanged half-float
rounding, production WGSL validation, GPU upload/readback, and GPU filtering.
