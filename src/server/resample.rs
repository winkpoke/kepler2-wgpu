//! 3D resampling for the ONNX segmentation path.
//!
//! The nnU-Net spine model was trained at a target spacing of
//! `[1.5, 1.5, 1.5]` mm (see `segment_engine.rs::ModelInfo::spacing_mm`).
//! Input CT volumes arrive at their native acquisition spacing, which can
//! differ widely (e.g. `[0.5, 0.5, 2.0]` for a typical chest CT). Running
//! inference without resampling feeds the network voxels at the wrong
//! physical scale and produces grossly incorrect masks.
//!
//! This module bridges that gap with:
//!
//! - [`resample_volume_cubic`]: cubic B-spline (`order=3`) resample of the
//!   `f32` HU volume to the model's target spacing, faithfully reproducing
//!   TotalSegmentator's `change_spacing(order=3)` CPU path, which routes
//!   through `scipy.ndimage.zoom(order=3, mode='nearest')`.
//!
//!   `scipy.ndimage.zoom(order=3)` (scipy ≥ 1.15) does THREE things the
//!   naive "cubic interpolation" does not:
//!     1. It *edge-pads* the input by `npad = 12` samples on each side
//!        (`_prepad_for_spline_filter`, `mode='nearest'` ⇒ `np.pad(mode=
//!        'edge')`).
//!     2. It *prefilters* that padded array with the cubic B-spline (β³)
//!        analysis filter (Unser's causal/anti-causal recursion,
//!        `pole = √3 − 2`), converting samples into B-spline coefficients.
//!     3. It evaluates the β³ basis at the output coordinates
//!        `coord[o] = o · (n_in − 1) / (n_out − 1) + npad`
//!        (scipy's default `grid_mode=False`: both endpoints map onto each
//!        other, so `zoom[0] == x[0]` and `zoom[-1] == x[-1]`).
//!
//!   The previous host implementation used a Keys `a = −0.5` kernel with no
//!   prefilter *and* the wrong coordinate convention (`o · n_in / n_out`),
//!   so it diverged from the Python reference by the full signal scale. The
//!   port below reproduces `scipy.ndimage.zoom(order=3, mode='nearest')` to
//!   machine precision (verified by
//!   `scripts/verify_preprocessing_alignment.py`).
//!
//! - [`upsample_logits_argmax`]: project the blended network **logits** from
//!   the 1.5 mm grid back onto the original voxel grid the way nnU-Net does,
//!   and argmax there. This is what actually decides the mask the viewer
//!   sees, and getting it wrong costs ~0.94 Dice against the Python
//!   reference (see the doc comment on the function).
//!
//! - [`resample_mask_nearest`]: legacy nearest-neighbour inverse resample of
//!   an already-argmaxed `u8` label mask. Kept as the fallback path when a
//!   backend does not hand back logits; it is measurably worse than
//!   [`upsample_logits_argmax`] and should not be used for the ONNX backend.
//!
//! No new dependencies: uses the already-present `rayon` crate for
//! slice-level parallelism in the mask inverse-resample.

use rayon::prelude::*;

/// Cubic B-spline pole `p = √3 − 2` (≈ −0.267949), used by the analysis
/// prefilter. Must match `scipy.ndimage.spline_filter1d(order=3)`.
const BSP_POLE: f64 = -0.2679491924311228; // √3 − 2

/// Edge padding applied before the spline prefilter, matching
/// `scipy.ndimage._interpolation._prepad_for_spline_filter` for
/// `mode='nearest'` (which uses `npad = 12`).
const PAD: usize = 12;

/// Nearest-neighbour inverse-resample a `u8` label mask from the resampled
/// grid back to the original voxel grid. Labels must never be interpolated
/// (a blend of label 3 and label 5 is meaningless), so nearest-neighbour is
/// the only correct choice.
///
/// `src_shape` is the resampled grid (where the mask was produced),
/// `tgt_shape` is the original grid (where the mask must land).
///
/// The source index is computed via the physical-space ratio
/// `src_idx = round(tgt_idx * src_dim / tgt_dim)`, which preserves the
/// mapping established by [`resample_volume_cubic`].
pub fn resample_mask_nearest(
    mask: &[u8],
    (sd, sh, sw): (usize, usize, usize),
    (td, th, tw): (usize, usize, usize),
) -> Vec<u8> {
    log::info!(
        "[AI/Resample] nearest mask {:?} -> {:?}",
        (sd, sh, sw), (td, th, tw),
    );

    let slices: Vec<Vec<u8>> = (0..td)
        .into_par_iter()
        .map(|oz| {
            let sz = scale_idx(oz, sd, td);
            let mut row = Vec::with_capacity(th * tw);
            for oy in 0..th {
                let sy = scale_idx(oy, sh, th);
                for ox in 0..tw {
                    let sx = scale_idx(ox, sw, tw);
                    row.push(mask[(sz * sh + sy) * sw + sx]);
                }
            }
            row
        })
        .collect();

    let mut out = Vec::with_capacity(td * th * tw);
    for slice in slices {
        out.extend_from_slice(&slice);
    }
    out
}

/// Nearest-neighbour index mapping: `round(tgt_i * src_n / tgt_n)`,
/// clamped to `[0, src_n - 1]`.
fn scale_idx(tgt_i: usize, src_n: usize, tgt_n: usize) -> usize {
    let f = (tgt_i as f64) * (src_n as f64) / (tgt_n as f64);
    let r = f.round();
    (r as usize).min(src_n.saturating_sub(1))
}

// ===========================================================================
// Cubic B-spline (β³) resample — faithful port of scipy.ndimage.zoom(order=3)
// ===========================================================================

/// β³ (cubic B-spline) basis weights for a fractional offset `x ∈ [0, 1)`.
///
/// Returns `[w0, w1, w2, w3]` for the four taps at positions
/// `i0−1, i0, i0+1, i0+2` respectively, where `i0 = floor(src_pos)`.
/// This is the standard cubic B-spline refinement kernel, **not** the
/// Keys `a = −0.5` kernel.
fn bspline3_weights(x: f64) -> [f64; 4] {
    let y = x;
    let z = 1.0 - x;
    let w1 = (y * y * (y - 2.0) * 3.0 + 4.0) / 6.0;
    let w2 = (z * z * (z - 2.0) * 3.0 + 4.0) / 6.0;
    let w0 = z * z * z / 6.0;
    let w3 = 1.0 - w0 - w1 - w2;
    [w0, w1, w2, w3]
}

/// Apply the 1-D cubic B-spline analysis prefilter (Unser) to a single line,
/// writing the resulting B-spline coefficients into `out`.
///
/// Mirrors `scipy.ndimage.spline_filter1d(order=3)`, whose pole recursion is
/// stable because `|pole| < 1`. Computed in `f64` for accuracy, then stored
/// back as `f32`.
fn spline_prefilter_line(line: &[f32], out: &mut [f32]) {
    let n = line.len();
    if n == 0 {
        return;
    }
    let mut c: Vec<f64> = line.iter().map(|&v| v as f64).collect();
    let z = BSP_POLE;
    let zn = z.powi(n as i32);
    let gain = (1.0 - z) * (1.0 - 1.0 / z);
    for v in c.iter_mut() {
        *v *= gain;
    }
    // Causal (forward) recursion.
    let mut s = c[0] + zn * c[n - 1];
    let mut zi = z;
    for i in 1..n {
        s += zi * (c[i] + zn * c[n - 1 - i]);
        zi *= z;
    }
    c[0] += s * z / (1.0 - zn * zn);
    for i in 1..n {
        c[i] += z * c[i - 1];
    }
    // Anti-causal (backward) recursion.
    c[n - 1] *= z / (z - 1.0);
    for i in (0..n - 1).rev() {
        c[i] = z * (c[i + 1] - c[i]);
    }
    for (i, v) in c.iter().enumerate() {
        out[i] = *v as f32;
    }
}

/// Edge-pad a `(Z, Y, X)` volume by `pad` samples on every side, replicating
/// the boundary voxel. Matches `np.pad(x, pad, mode='edge')`, which is what
/// scipy's `_prepad_for_spline_filter(mode='nearest')` does.
fn pad_edge(
    data: &[f32],
    (d, h, w): (usize, usize, usize),
    pad: usize,
) -> (Vec<f32>, (usize, usize, usize)) {
    let (pd, ph, pw) = (d + 2 * pad, h + 2 * pad, w + 2 * pad);
    let mut out = vec![0.0f32; pd * ph * pw];
    for z in 0..pd {
        let sz = (z as isize - pad as isize).clamp(0, d as isize - 1) as usize;
        for y in 0..ph {
            let sy = (y as isize - pad as isize).clamp(0, h as isize - 1) as usize;
            for x in 0..pw {
                let sx = (x as isize - pad as isize).clamp(0, w as isize - 1) as usize;
                out[(z * ph + y) * pw + x] = data[(sz * h + sy) * w + sx];
            }
        }
    }
    (out, (pd, ph, pw))
}

/// Convert a whole `(Z, Y, X)` volume into B-spline coefficients along a
/// single axis.
///
/// `axis`: `0 = Z`, `1 = Y`, `2 = X`. The prefilter runs independently on
/// every 1-D line along `axis`.
fn prefilter_axis(
    data: &[f32],
    (d, h, w): (usize, usize, usize),
    axis: usize,
) -> Vec<f32> {
    let mut out = vec![0.0f32; d * h * w];
    match axis {
        2 => {
            // X lines: contiguous in memory.
            for i in 0..d {
                for j in 0..h {
                    let base = (i * h + j) * w;
                    let line = &data[base..base + w];
                    let mut coeff = vec![0.0f32; w];
                    spline_prefilter_line(line, &mut coeff);
                    out[base..base + w].copy_from_slice(&coeff);
                }
            }
        }
        1 => {
            // Y lines: stride `w`.
            for i in 0..d {
                for j in 0..w {
                    let mut line = vec![0.0f32; h];
                    let mut coeff = vec![0.0f32; h];
                    for y in 0..h {
                        line[y] = data[(i * h + y) * w + j];
                    }
                    spline_prefilter_line(&line, &mut coeff);
                    for y in 0..h {
                        out[(i * h + y) * w + j] = coeff[y];
                    }
                }
            }
        }
        _ => {
            // Z lines: stride `h * w`.
            for j in 0..h {
                for k in 0..w {
                    let mut line = vec![0.0f32; d];
                    let mut coeff = vec![0.0f32; d];
                    for z in 0..d {
                        line[z] = data[(z * h + j) * w + k];
                    }
                    spline_prefilter_line(&line, &mut coeff);
                    for z in 0..d {
                        out[(z * h + j) * w + k] = coeff[z];
                    }
                }
            }
        }
    }
    out
}

/// Evaluate the β³ basis along a single axis (`0=Z, 1=Y, 2=X`) of a
/// row-major volume, resampling it from `new_size` output samples.
///
/// `data` is the (already edge-padded and prefiltered) volume; its axis size
/// along `axis` is the padded size. `old_orig` is the ORIGINAL (unpadded)
/// size along `axis`, used for scipy's coordinate convention
/// `pos = o · (old_orig − 1) / (new_size − 1) + PAD`. Out-of-range taps
/// clamp to the nearest edge voxel, matching scipy `mode='nearest'`.
fn resample_axis_bspline(
    data: &[f32],
    (d, h, w): (usize, usize, usize),
    axis: usize,
    new_size: usize,
    old_orig: usize,
) -> Vec<f32> {
    let padded_size = match axis {
        0 => d,
        1 => h,
        _ => w,
    };
    let new_d = if axis == 0 { new_size } else { d };
    let new_h = if axis == 1 { new_size } else { h };
    let new_w = if axis == 2 { new_size } else { w };
    let mut out = vec![0.0f32; new_d * new_h * new_w];

    let i_bound = if axis == 0 { h } else { d };
    let j_bound = if axis == 2 { h } else { w };
    let span = if new_size > 1 {
        (old_orig.saturating_sub(1)) as f64 / (new_size - 1) as f64
    } else {
        0.0
    };

    for i in 0..i_bound {
        for j in 0..j_bound {
            for o in 0..new_size {
                let pos = if new_size > 1 {
                    o as f64 * span + PAD as f64
                } else {
                    PAD as f64
                };
                let i0 = pos.floor() as isize;
                let x = pos - i0 as f64;
                let wts = bspline3_weights(x);
                let mut acc = 0.0f64;
                for k in 0..4 {
                    let mut idx = i0 + (k as isize) - 1;
                    if idx < 0 {
                        idx = 0;
                    } else if idx >= padded_size as isize {
                        idx = padded_size as isize - 1;
                    }
                    let (zz, yy, xx) = match axis {
                        0 => (idx as usize, i, j),
                        1 => (i, idx as usize, j),
                        _ => (i, j, idx as usize),
                    };
                    acc += data[(zz * h + yy) * w + xx] as f64 * wts[k];
                }
                let (zz, yy, xx) = match axis {
                    0 => (o, i, j),
                    1 => (i, o, j),
                    _ => (i, j, o),
                };
                out[(zz * new_h + yy) * new_w + xx] = acc as f32;
            }
        }
    }
    out
}

/// Cubic (β³) separable resample matching
/// `scipy.ndimage.zoom(order=3, mode='nearest')` exactly.
///
/// TotalSegmentator resamples the HU image with `change_spacing(...,
/// order=3, dtype=np.int32)`, which routes through `scipy.ndimage.zoom`
/// (CPU path). Replicating the full order-3 pipeline — edge pad (12) +
/// B-spline prefilter + β³ evaluation at `o·(n_in−1)/(n_out−1)+npad` —
/// removes the dominant interpolation偏差 between the ONNX path and the
/// Python reference (verified to machine precision).
///
/// Output shape per axis is `round(n_in · src_spacing / tgt_spacing)`, same
/// as `zoom`.
pub fn resample_volume_cubic(
    data: &[f32],
    (sd, sh, sw): (usize, usize, usize),
    src_spacing: [f32; 3],
    tgt_spacing: [f32; 3],
) -> (Vec<f32>, (usize, usize, usize)) {
    let od = ((sd as f32 * src_spacing[0] / tgt_spacing[0]).round() as usize).max(1);
    let oh = ((sh as f32 * src_spacing[1] / tgt_spacing[1]).round() as usize).max(1);
    let ow = ((sw as f32 * src_spacing[2] / tgt_spacing[2]).round() as usize).max(1);

    // Step 1: edge-pad by PAD on every axis (scipy _prepad_for_spline_filter).
    let (padded, (pd, ph, pw)) = pad_edge(data, (sd, sh, sw), PAD);

    // Step 2+3: separable — per axis, prefilter into B-spline coefficients,
    // then evaluate/decimate along that axis. Intermediate axes keep their
    // padding until they are resampled; `old_orig` is the ORIGINAL size.
    let c_x = prefilter_axis(&padded, (pd, ph, pw), 2);
    let a_x = resample_axis_bspline(&c_x, (pd, ph, pw), 2, ow, sw);

    let c_y = prefilter_axis(&a_x, (pd, ph, ow), 1);
    let a_y = resample_axis_bspline(&c_y, (pd, ph, ow), 1, oh, sh);

    let c_z = prefilter_axis(&a_y, (pd, oh, ow), 0);
    let a_z = resample_axis_bspline(&c_z, (pd, oh, ow), 0, od, sd);

    (a_z, (od, oh, ow))
}

// ===========================================================================
// Logits -> native grid projection (nnU-Net `resampling_fn_probabilities`)
// ===========================================================================

/// nnU-Net's `ANISO_THRESHOLD` (`nnunetv2/configuration.py`). A spacing whose
/// max/min ratio exceeds this is resampled with the low-resolution axis
/// handled separately (`order_z = 0`, i.e. nearest neighbour).
const ANISO_THRESHOLD: f32 = 3.0;

/// Mirror of nnU-Net's `determine_do_sep_z_and_axis(None, current, new, 3.0)`
/// followed by `get_lowres_axis`.
///
/// `current` is the grid being sampled FROM (the model's target spacing,
/// `[1.5, 1.5, 1.5]`) and `new` is the grid being sampled TO (the native
/// spacing of the volume). Returns `Some(axis)` when that axis must be
/// nearest-neighbour interpolated, `None` when every axis is linear.
///
/// nnU-Net also disables separate-z when the low-res axis is not unique
/// (`len(axis) != 1`); we express that by returning `None`.
pub fn nnunet_aniso_axis(current: [f32; 3], new: [f32; 3]) -> Option<usize> {
    let anisotropy = |s: &[f32; 3]| -> f32 {
        let mx = s.iter().copied().fold(f32::MIN, f32::max);
        let mn = s.iter().copied().fold(f32::MAX, f32::min);
        if mn > 0.0 { mx / mn } else { 1.0 }
    };
    // `get_lowres_axis`: the axis whose spacing equals the maximum. Returns
    // `None` unless exactly one axis qualifies (nnU-Net then bails out of
    // separate-z, matching our `None`).
    let lowres = |s: &[f32; 3]| -> Option<usize> {
        let mx = s.iter().copied().fold(f32::MIN, f32::max);
        let hits: Vec<usize> = (0..3)
            .filter(|&i| (mx - s[i]).abs() <= 1e-6 * mx.max(1.0))
            .collect();
        if hits.len() == 1 { Some(hits[0]) } else { None }
    };
    if anisotropy(&current) > ANISO_THRESHOLD {
        lowres(&current)
    } else if anisotropy(&new) > ANISO_THRESHOLD {
        lowres(&new)
    } else {
        None
    }
}

/// Per-output-index interpolation taps along one axis: read
/// `v[i0] * (1 - f) + v[i1] * f`, where `i0 == i1` and `f == 0` for a
/// nearest-neighbour (order-0) axis.
struct AxisMap {
    i0: Vec<usize>,
    i1: Vec<usize>,
    f: Vec<f32>,
}

impl AxisMap {
    /// `nearest == true` reproduces `scipy.ndimage.map_coordinates(order=0)`;
    /// otherwise it reproduces `skimage.transform.resize(order=1,
    /// mode='edge', anti_aliasing=False)`.
    ///
    /// Both use the same coordinate convention (`align_corners=False`):
    /// `x = (o + 0.5) * src_n / tgt_n - 0.5`, with out-of-range coordinates
    /// clamped to `[0, src_n - 1]` (skimage's `mode='edge'` is ndimage's
    /// `mode='nearest'`). `order=0` then rounds with `floor(x + 0.5)`.
    fn new(src_n: usize, tgt_n: usize, nearest: bool) -> Self {
        debug_assert!(src_n > 0 && tgt_n > 0);
        let last = (src_n - 1) as f64;
        let mut m = AxisMap {
            i0: Vec::with_capacity(tgt_n),
            i1: Vec::with_capacity(tgt_n),
            f: Vec::with_capacity(tgt_n),
        };
        for o in 0..tgt_n {
            let x = (o as f64 + 0.5) * (src_n as f64) / (tgt_n as f64) - 0.5;
            let xc = x.clamp(0.0, last);
            if nearest {
                let idx = ((xc + 0.5).floor() as isize).clamp(0, src_n as isize - 1) as usize;
                m.i0.push(idx);
                m.i1.push(idx);
                m.f.push(0.0);
            } else {
                let fl = xc.floor();
                let idx = fl as usize;
                m.i0.push(idx);
                m.i1.push((idx + 1).min(src_n - 1));
                m.f.push((xc - fl) as f32);
            }
        }
        m
    }
}

/// Project blended network logits from the resampled grid back onto the
/// original voxel grid, exactly the way nnU-Net does, and argmax there.
///
/// nnU-Net does **not** resample the argmaxed label map. `plans.json` stores
///
/// ```text
/// resampling_fn_probabilities_kwargs =
///     {'is_seg': False, 'order': 1, 'order_z': 0, 'force_separate_z': None}
/// ```
///
/// and `nnunetv2/inference/export_prediction.py` feeds the *logits* through
/// that function before `label_manager.convert_logits_to_segmentation`
/// (an argmax). Concretely:
///
/// 1. every axis is linearly interpolated at
///    `x = (o + 0.5) * src / tgt - 0.5` with edge clamping
///    (`skimage.transform.resize(order=1, mode='edge')`, i.e.
///    `align_corners=False`),
/// 2. **unless** the spacing is anisotropic (max/min > 3). Then the
///    low-resolution axis is excluded from `resize` and re-sampled with
///    `map_coordinates(order=0, mode='nearest')` — plain nearest neighbour
///    (see [`nnunet_aniso_axis`]),
/// 3. and only then is the argmax taken.
///
/// Resampling the *labels* with nearest neighbour instead (what this module
/// used to do via [`resample_mask_nearest`]) can only place a boundary on the
/// coarse grid. Measured on a real spine CT, that alone costs ~0.94 Dice on
/// every foreground label (worst 0.9371) versus the reference, because the
/// in-plane upsampling factor is close to 5x while the label grid is 1.5 mm.
///
/// `logits` is `[C, sd, sh, sw]` row-major. The returned buffer is the label
/// map at `(td, th, tw)` in the same row-major order, holding the raw channel
/// index of the winner (the caller collapses the background / "rest"
/// channels).
pub fn upsample_logits_argmax(
    logits: &[f32],
    num_classes: usize,
    (sd, sh, sw): (usize, usize, usize),
    (td, th, tw): (usize, usize, usize),
    aniso_axis: Option<usize>,
) -> Vec<u8> {
    assert_eq!(
        logits.len(),
        num_classes * sd * sh * sw,
        "logits length {} != C*D*H*W = {}*{sd}*{sh}*{sw}",
        logits.len(),
        num_classes
    );
    assert!(num_classes > 0 && sd > 0 && sh > 0 && sw > 0);
    assert!(td > 0 && th > 0 && tw > 0);

    let zmap = AxisMap::new(sd, td, aniso_axis == Some(0));
    let ymap = AxisMap::new(sh, th, aniso_axis == Some(1));
    let xmap = AxisMap::new(sw, tw, aniso_axis == Some(2));
    // When the axis-0 map is order-0 (or the depth is unchanged) the two
    // z-taps are the same sample, so the second set of reads is redundant.
    let z_is_nearest = aniso_axis == Some(0) || sd == td;

    // In-plane bilinear weights are identical for every slice; hoist them.
    let mut w_tab = Vec::with_capacity(th * tw);
    for oy in 0..th {
        for ox in 0..tw {
            let fy = ymap.f[oy];
            let fx = xmap.f[ox];
            w_tab.push([
                (1.0 - fy) * (1.0 - fx),
                (1.0 - fy) * fx,
                fy * (1.0 - fx),
                fy * fx,
            ]);
        }
    }

    let shw = sh * sw;
    let plane = th * tw;

    log::info!(
        "[AI/Resample] logits projection {:?} -> {:?} (C={}, aniso_axis={:?})",
        (sd, sh, sw), (td, th, tw), num_classes, aniso_axis
    );

    let rows: Vec<Vec<u8>> = (0..td)
        .into_par_iter()
        .map(|oz| {
            let z0 = zmap.i0[oz];
            let z1 = zmap.i1[oz];
            let fz = zmap.f[oz];

            let mut buf = vec![0.0f32; num_classes * plane];
            for c in 0..num_classes {
                let base = c * sd * shw;
                let b0 = base + z0 * shw;
                let b1 = base + z1 * shw;
                let cb = c * plane;
                for oy in 0..th {
                    let y0r = b0 + ymap.i0[oy] * sw;
                    let y1r = b0 + ymap.i1[oy] * sw;
                    let (y0r_1, y1r_1) = if z_is_nearest {
                        (y0r, y1r) // unused
                    } else {
                        (b1 + ymap.i0[oy] * sw, b1 + ymap.i1[oy] * sw)
                    };
                    for ox in 0..tw {
                        let x0 = xmap.i0[ox];
                        let x1 = xmap.i1[ox];
                        let w = w_tab[oy * tw + ox];
                        let v0 = w[0] * logits[y0r + x0]
                            + w[1] * logits[y0r + x1]
                            + w[2] * logits[y1r + x0]
                            + w[3] * logits[y1r + x1];
                        let v = if z_is_nearest {
                            v0
                        } else {
                            let v1 = w[0] * logits[y0r_1 + x0]
                                + w[1] * logits[y0r_1 + x1]
                                + w[2] * logits[y1r_1 + x0]
                                + w[3] * logits[y1r_1 + x1];
                            v0 + fz * (v1 - v0)
                        };
                        buf[cb + oy * tw + ox] = v;
                    }
                }
            }

            let mut row = vec![0u8; plane];
            for i in 0..plane {
                let mut best = f32::MIN;
                let mut best_c = 0u8;
                for c in 0..num_classes {
                    let v = buf[c * plane + i];
                    if v > best {
                        best = v;
                        best_c = c as u8;
                    }
                }
                row[i] = best_c;
            }
            row
        })
        .collect();

    let mut out = Vec::with_capacity(td * plane);
    for row in rows {
        out.extend_from_slice(&row);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nnunet_aniso_axis_matches_reference() {
        // The real case: 1.5mm model spacing -> 1.0/0.3125/0.3125 native
        // (ratio 3.2 > ANISO_THRESHOLD), low-res axis is 0 (head-foot).
        assert_eq!(
            nnunet_aniso_axis([1.5, 1.5, 1.5], [1.0, 0.3125, 0.3125]),
            Some(0)
        );
        // Isotropic on both sides -> every axis linear.
        assert_eq!(nnunet_aniso_axis([1.5, 1.5, 1.5], [1.0, 1.0, 1.0]), None);
        // Anisotropic on the CURRENT grid picks its own low-res axis.
        assert_eq!(nnunet_aniso_axis([0.7, 0.7, 5.0], [0.7, 0.7, 5.0]), Some(2));
        // Anisotropic only on the NEW grid.
        assert_eq!(nnunet_aniso_axis([1.5, 1.5, 1.5], [0.4, 0.4, 2.0]), Some(2));
        // Ratio exactly at the threshold is NOT anisotropic (strict >).
        assert_eq!(nnunet_aniso_axis([1.0, 1.0, 1.0], [1.0, 1.0, 3.0]), None);
    }

    #[test]
    fn upsample_is_identity_when_grids_match() {
        let src: Vec<f32> = (0..8).map(|v| v as f32).collect(); // (1,2,2,2)
        let out = upsample_logits_argmax(&src, 1, (2, 2, 2), (2, 2, 2), None);
        assert_eq!(out, vec![0u8; 8]);
    }

    /// Discriminates linear (order=1) from nearest (order=0) along axis 2.
    ///
    /// Source columns (sw=4) hold ch0 = [0, 1, 2, 3] and the constant
    /// ch1 = 1.1. With `tw=8` the coordinates are
    /// `x = (o+0.5)*0.5 - 0.5` -> [-0.25, 0.25, 0.75, 1.25, 1.75, 2.25,
    /// 2.75, 3.25], clamped to [0, 3] at both ends.
    ///
    /// * linear ch0 -> [0, .25, .75, 1.25, 1.75, 2.25, 2.75, 3], so ch0 only
    ///   overtakes 1.1 at o=3 -> labels [1, 1, 1, 0, 0, 0, 0, 0];
    /// * nearest copies whole columns -> [0, 0, 1, 1, 2, 2, 3, 3], so ch0
    ///   overtakes 1.1 only at o=4 -> labels [1, 1, 1, 1, 0, 0, 0, 0].
    #[test]
    fn upsample_linear_vs_nearest_differs() {
        let logits = vec![0.0, 1.0, 2.0, 3.0, /* ch1 */ 1.1, 1.1, 1.1, 1.1];
        let linear = upsample_logits_argmax(&logits, 2, (1, 1, 4), (1, 1, 8), None);
        let nearest = upsample_logits_argmax(&logits, 2, (1, 1, 4), (1, 1, 8), Some(2));
        assert_eq!(linear, vec![1, 1, 1, 0, 0, 0, 0, 0]);
        assert_eq!(nearest, vec![1, 1, 1, 1, 0, 0, 0, 0]);
    }

    #[test]
    fn upsample_argmaxes_over_all_channels() {
        // Channel 5 must win everywhere; channel 0 is the default loser.
        let mut two = vec![0.0f32; 2 * 2 * 2 * 2];
        for i in 0..8 {
            two[i] = -1.0;
            two[8 + i] = 1.0;
        }
        let out = upsample_logits_argmax(&two, 2, (2, 2, 2), (4, 4, 4), None);
        assert_eq!(out.len(), 64);
        assert!(out.iter().all(|&v| v == 1));
    }

    /// Fixture-based proof on REAL data: the shipped projection must be
    /// byte-identical to nnU-Net's `resampling_fn_probabilities` output.
    ///
    /// `scripts/_diag_upsample.py` + `scripts/_dump_fixtures.py` produce
    /// `_diag_logits15.raw` ([9, 160, 107, 107] f32 LE) and `_diag_a2.raw`
    /// ([240, 512, 512] u8: resample the logits with order=1 in-plane /
    /// order_z=0, argmax, collapse the "rest" channel to 0). Both are
    /// machine-local and gitignored, so this test is `#[ignore]`d:
    ///
    /// ```text
    /// cargo test --lib -- --ignored upsample_logits_reference_fixture
    /// ```
    ///
    /// Without the fixtures it prints a SKIP line and passes, so CI stays
    /// green.
    #[test]
    #[ignore = "needs machine-local scripts/_diag_*.raw fixtures"]
    fn upsample_logits_reference_fixture() {
        use std::path::Path;
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts");
        let lg_path = dir.join("_diag_logits15.raw");
        let ref_path = dir.join("_diag_a2.raw");
        if !lg_path.exists() || !ref_path.exists() {
            eprintln!("SKIP upsample_logits_reference_fixture: fixtures missing");
            return;
        }
        let raw = std::fs::read(&lg_path).expect("read logits fixture");
        let logits: Vec<f32> = raw
            .chunks_exact(4)
            .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
            .collect();
        let expected = std::fs::read(&ref_path).expect("read reference fixture");

        // 9 channels: 0 = background, 1..7 = spine, 8 = "rest".
        let mut got =
            upsample_logits_argmax(&logits, 9, (160, 107, 107), (240, 512, 512), Some(0));
        for v in got.iter_mut() {
            if *v == 8 {
                *v = 0;
            }
        }

        assert_eq!(got.len(), expected.len(), "mask size mismatch");
        let mismatched = got
            .iter()
            .zip(expected.iter())
            .filter(|(a, b)| a != b)
            .count();
        assert_eq!(
            mismatched, 0,
            "{mismatched} of {} voxels differ from the nnU-Net reference",
            got.len()
        );
        eprintln!(
            "OK upsample_logits_reference_fixture: {} voxels identical to the reference",
            got.len()
        );
    }
}
