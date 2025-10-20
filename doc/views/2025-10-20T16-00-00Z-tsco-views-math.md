# TSCO Views: Math Summary

This note summarizes the math used to construct and navigate multi-planar (TSCO) views — Transverse (Axial), Sagittal, Coronal, and Oblique — in the codebase.

The rendering pipeline maps a 2D screen quad and a slice parameter to 3D texture UV coordinates using orientation-specific bases and a composed screen→UV transform.

## Coordinate Systems
- Screen: `x ∈ [0,1]`, `y ∈ [0,1]` (top-left origin), `z` is the slice parameter in "screen units".
- Volume indices: `(i, j, k)` voxel indices.
- Patient/world: DICOM patient coordinates (LR, AP, SI) via `vol.base.matrix`.
- UV: Normalized texture coordinates `(u, v, w) ∈ [0,1]^3`.

## Notation
- Dimensions: `nx, ny, nz` voxels.
- Spacing: `sx, sy, sz` millimeters per voxel.
- Physical extents: `d_x = nx * sx`, `d_y = ny * sy`, `d_z = nz * sz`.
- Isotropic in-plane choice (display): `d = max(d_x, d_y)`.
- Volume origin/offset (patient/world): `[ox, oy, oz] = vol.base.matrix.get_column(3)[0..=2]`.

## UV Base (Normalized → World)
Builds a base that maps normalized UV to the volume’s patient/world space:
- Scale UV by index extents (note: counts → index range `N-1`):
  - `S = diag(nx-1, ny-1, nz-1, 1)`
- World matrix: `M_world = vol.base.matrix * S`
- Stored as `build_uv_base(vol)`.

## Screen→UV Transform (Shader `mat`)
- Start with orientation-specific `base_screen` (Screen → World).
- Apply view transforms on `base_screen` in this order:
  - `translate([-pan.x, -pan.y, -pan.z])`
  - `translate([0.5, 0.5, 0.0])`
  - `scale([scale, scale, 1.0])`
  - `translate([-0.5, -0.5, 0.0])`
- Convert to UV space: `mat = base_screen.to_base(base_uv).transpose()`.
- Fragment shader uses:
  - `tex_coords_3d = (mat * vec4(x_screen, y_screen, slice, 1)).xyz`

Slice parameter (`slice`) should be in the same "screen units" used by `base_screen`. Millimeter values are converted via `set_slice_mm`.

## Scale Factors and mm Conversion
- `base_screen.get_scale_factors()` returns `[s_x, s_y, s_z]` as norms of the first three columns.
- Millimeters to screen units:
  - Pan: `pan_x = x_mm / s_x`, `pan_y = y_mm / s_y`
  - Slice: `pan_z = z_mm / s_z`
- In code: `set_pan_mm(x_mm, y_mm)`, `set_slice_mm(z_mm)` implement the above.

## Orientation Bases (Screen → World)
Matrices are given row-wise (row-major). Each of the first three rows corresponds to world X, Y, Z; columns correspond to screen X, Y, Z respectively; last column is translation.

### Transverse (Axial)
- In-plane: LR (world X), AP (world Y)
- Slice: SI (world Z)
- Isotropic in-plane choice:
```
[  d,  0,  0,  ox ]  // world X ← screen X
[  0,  d,  0,  oy ]  // world Y ← screen Y
[  0,  0, dz,  oz ]  // world Z ← screen Z (slice)
[  0,  0,  0,   1 ]
```
Where `d = max(d_x, d_y)` and `dz = d_z`. For mm-true aspect in-plane, use `d_x` and `d_y` instead of `d`.

### Coronal
- In-plane: LR (world X), SI (world Z)
- Slice: AP (world Y)
- mm-true mapping:
```
[ d_x,   0,   0,            ox ]  // world X ← screen X
[   0,   0, d_y,  oy + d_y/2.0 ]  // world Y ← screen Z (slice), centered
[   0, -d_z,   0,  oz + d_z/2.0 ]  // world Z ← screen Y (Y-down inversion), centered
[   0,   0,   0,             1 ]
```
Notes:
- Use `d_y` for the slice normal (AP) axis.
- Use `-d_z` for screen Y to world Z to account for screen Y-down.
- Half-extent translations center the volume along slice and SI axes.

### Sagittal
- In-plane: AP (world Y), SI (world Z)
- Slice: LR (world X)
- mm-true mapping:
```
[   0,   0, d_x,  ox + d_x/2.0 ]  // world X ← screen Z (slice), centered
[ d_y,   0,   0,  oy + d_y/2.0 ]  // world Y ← screen X, centered
[   0, -d_z,   0,  oz + d_z/2.0 ]  // world Z ← screen Y (Y-down inversion), centered
[   0,   0,   0,             1 ]
```
Notes:
- Use `d_x` for the slice normal (LR) axis.
- In-plane uses `d_y` (AP) and `d_z` (SI) with Y-down inversion.

### Oblique (Generic)
Oblique views are defined by an orthonormal basis `{u, v, w}` in patient space:
- `w`: slice normal (unit vector in world coordinates).
- `u, v`: in-plane orthonormal axes (unit vectors), with `v` chosen so that screen Y points down (apply a sign flip in the Z-row equivalent).
- Let in-plane physical extents be `s_u, s_v` and slice extent `s_w` (mm).
- Screen→World mapping:
```
M = [ u.x*s_u  v.x*(-s_v)  w.x*s_w  ox + c_x
      u.y*s_u  v.y*(-s_v)  w.y*s_w  oy + c_y
      u.z*s_u  v.z*(-s_v)  w.z*s_w  oz + c_z
      0        0           0        1       ]
```
Where `(c_x, c_y, c_z)` apply centering along the axes you want centered (commonly half-extent on the slice axis).

You can derive `{u, v, w}` from DICOM image orientation vectors or from a desired plane specification, then normalize and orthogonalize.

## Practical Considerations
- Isotropic display (`d = max(d_x, d_y)`) gives round pixels on screen; mm-true in-plane uses `d_x, d_y` separately.
- Always use the true physical extent for the slice axis (`d_y` for coronal, `d_x` for sagittal, `d_z` for transverse).
- Screen Y-down is handled by a negative scale on the row mapping screen Y.
- Centering terms (`+ half-extent`) align the slice origin to the middle of the physical extent along that axis; adjust per UX need.

## Shader Path Recap
- Fragment shader multiplies a 4D vector containing screen coords and `slice` by `mat` and samples the 3D texture:
  - `tex_coords_3d = (mat * vec4(x, y, slice, 1)).xyz`
- Out-of-bounds checks clamp sampling to `[0,1]^3`.
- Window/level mapping is applied after sampling, with optional packed RG8 decoding.

## Debug/Validation Tips
- Log `base_screen.get_scale_factors()` to verify `[s_x, s_y, s_z]` match expected `[in-plane_x_mm, in-plane_y_mm, slice_mm]` for the given orientation.
- Move a known millimeter distance via `set_pan_mm` and `set_slice_mm` and confirm that positions in the dataset are consistent.
- For oblique, orthonormality of `{u, v, w}` is critical; re-orthogonalize if needed and ensure `det([u v w]) > 0` for right-handed systems.