# Suggestions for generate_ct_volume_common and CT volume pipeline

This document captures actionable suggestions to improve the correctness and robustness of CT volume generation and rendering.

## 1) Base matrix composition order
- Compose the voxel-to-patient transform as Translation · Direction · Scaling, i.e., `M = T * R * S`.
- Rationale: Image Position (Patient) is given in patient coordinates and must not be rotated by the direction matrix.
- Implementation notes:
  - Direction matrix columns should be `[row_dir, col_dir, slice_dir]` (each a unit vector).
  - Scaling matrix should apply voxel spacing per axis.
  - Verify `Matrix4x4::multiply` semantics and operator `*` to maintain left-to-right order.

## 2) Texture upload width/height mapping
- Use `width = columns`, `height = rows`, `depth = num_slices` when creating the 3D texture.
- Ensure `bytes_per_row = 2 * columns` for `Rg8Unorm` (two bytes per sample).
- Update the call site in the loader so `RenderContent::from_bytes(..., width = vol.dimensions.1, height = vol.dimensions.0, depth = vol.dimensions.2)`.

## 3) Robust slice spacing fallback
- If `Spacing Between Slices` is missing, compute slice spacing from geometry:
  - `slice_spacing = abs(dot(ImagePosition(next) - ImagePosition(curr), slice_direction))`.
  - This works for oblique stacks and non-axial orientations.
- Consider sorting slices by `dot(ImagePosition, slice_direction)` rather than the raw z component.

## 4) PixelSpacing semantics and voxel_spacing mapping
- PixelSpacing is `[row spacing (mm), column spacing (mm)]`. Map to `voxel_spacing = (row_spacing, column_spacing, slice_spacing)`.
- Verify consistent usage across geometry builders and UI (e.g., transverse/coronal/sagittal bases).

## 5) Axis mapping and shader expectations
- Texture X corresponds to DICOM columns (j), Y to rows (i), Z to slice index (s).
- The shader uniform `mat` should transform normalized `[u,v,depth]` into `[0,1]^3` texture coordinates.
- Confirm `build_uv_base` scales `[0,1]^3` to `[0..rows-1, 0..columns-1, 0..slices-1]` before applying the volume base.

## 6) Validate and normalize direction cosines
- Enforce orthonormality: normalize `row_direction` and `column_direction`, ensure `dot(row, col) ≈ 0`.
- Compute `slice_direction = cross(row_direction, column_direction)` and normalize.
- Log/warn if the input deviates beyond a tolerance (e.g., 1e-5).

## 7) Consistency checks across the series
- Ensure all slices share identical rows/columns.
- Check that orientation vectors remain consistent across the series; tolerate minor rounding errors.
- Verify monotonic progression of slice indices along `slice_direction`.

## 8) Pixel decoding and packing
- In `CTImage::get_pixel_data`, correctly handle signed vs. unsigned 16-bit, rescale slope/intercept.
- When packing to `Rg8Unorm`, map low byte to R and high byte to G.
- Maintain a consistent offset (e.g., HU_OFFSET) only at one stage of the pipeline.

## 9) Matrix math hygiene
- Confirm `multiply` is row-major and implements standard multiplication.
- Be explicit with multiply order to avoid unintended transposition; prefer `a.multiply(&b)` over chained `*` when clarity is needed.

## 10) Future enhancements
- Support multi-frame DICOM (enhanced CT), per-frame functional groups.
- Optional resampling to isotropic spacing for consistent MPR.
- Partial-volume blending or tri-linear sampling toggle for quality vs. performance.

## Action items (locations)
- Update base composition in `src/dicom/dicom_repo.rs` within `generate_ct_volume_common`.
- Update texture dimension mapping in `src/state.rs` within `load_data_from_ct_volume`.
- Improve slice spacing fallback in `src/dicom/dicom_repo.rs` using projection onto `slice_direction`.
- Audit `src/geometry.rs` builders to ensure spacing and axis mapping align with the above.