//! RAS canonicalization to match TotalSegmentator / nibabel
//! `as_closest_canonical`.
//!
//! TotalSegmentator's `nnunet.py` calls `as_closest_canonical(img_in)` on the
//! resampled image before running nnU-Net, and `undo_canonical` afterwards so
//! the final mask lands back in the original image space. The ONNX host must
//! reproduce the *forward* reorientation so the network sees a RAS-canonical
//! volume (what it was trained on), then apply the *inverse* so the emitted
//! mask overlays the original MHA exactly like the Python backend.
//!
//! Coordinate bookkeeping:
//! - The MHA parser yields a `Z,Y,X` row-major `f32` volume (host "axis 0 = Z,
//!   1 = Y, 2 = X"). The MHA `orientation` 3x3 direction-cosines matrix is
//!   `orientation[c][w]` where `c` is the image axis (0 = i = x, 1 = j = y,
//!   2 = k = z) and `w` is the world axis.
//! - A host array axis `a` (0=Z,1=Y,2=X) therefore carries image axis `2 - a`,
//!   so its world direction is `orientation[2 - a][w]`.
//! - For each canonical world axis `w` we pick the host axis `a` whose
//!   direction dominates (`argmax_a |orientation[2-a][w]|`); a negative sign
//!   means that axis must be flipped. This is exactly what nibabel's
//!   `io_orientation` / `as_closest_canonical` compute — **provided** the
//!   matrix is expressed in nibabel's world convention first.
//!
//! World convention (this is the trap): ITK / MHA store direction cosines in
//! **LPS** world (`x = Left, y = Posterior, z = Superior`), whereas nibabel /
//! nnU-Net use **RAS** (`x = Right, y = Anterior, z = Superior`). The two
//! differ by `S = diag(-1, -1, 1)`, so every world component `w < 2` must be
//! negated before deriving the plan. `orientation[c][w] = D_itk[w][c]`
//! (verified empirically: the flat `TransformMatrix` is the *column-major*
//! flattening of ITK's direction matrix), and nibabel's affine rotation is
//! `A_rot = S · D_itk`, so the matrix nibabel's `io_orientation` would see is
//! `A_rotᵀ = D_itkᵀ · S = orientation · S` — i.e. `orientation` with its two
//! first world columns negated. Skipping this negation silently drops the
//! in-plane flips and the network classifies everything as background.

/// Plan describing how to move between original (Z,Y,X) and canonical
/// (R,A,S) = (X,Y,Z) layouts.
#[derive(Clone)]
pub struct ReorientPlan {
    /// New (canonical) axis `w` is sourced from old (host) axis `perm[w]`
    /// (0=Z, 1=Y, 2=X).
    pub perm: [usize; 3],
    /// Flip canonical axis `w` if true.
    pub flip: [bool; 3],
    /// Original MHA volume shape in `(Z, Y, X)` order.
    pub original_shape: (usize, usize, usize),
}

impl ReorientPlan {
    /// Build the plan from an MHA 3x3 direction-cosines matrix
    /// `orientation[c][w]` (image axis `c` ∈ {0=i,1=j,2=k}, world axis
    /// `w` ∈ {0,1,2}).
    ///
    /// The incoming matrix is in ITK/LPS world; it is converted to nibabel's
    /// RAS world (negate the two first world columns) before the dominant
    /// axis / sign search, so the resulting plan reproduces nibabel's
    /// `as_closest_canonical` exactly.
    pub fn from_orientation(orientation: [[f32; 3]; 3]) -> Self {
        // LPS -> RAS: components along world x (Left) and y (Posterior)
        // flip sign in the RAS frame.
        let ras = |c: usize, w: usize| -> f32 {
            if w < 2 {
                -orientation[c][w]
            } else {
                orientation[c][w]
            }
        };

        let mut perm = [0usize; 3];
        let mut flip = [false; 3];
        for w in 0..3 {
            let mut best_a = 0usize;
            let mut best_abs = -1.0f32;
            for a in 0..3 {
                let val = ras(2 - a, w);
                if val.abs() > best_abs {
                    best_abs = val.abs();
                    best_a = a;
                }
            }
            perm[w] = best_a;
            flip[w] = ras(2 - best_a, w) < 0.0;
        }
        ReorientPlan {
            perm,
            flip,
            original_shape: (0, 0, 0),
        }
    }

    /// Fold nnU-Net's final axis reversal into the plan.
    ///
    /// `NibabelIOWithReorient` canonicalizes to RAS and then applies
    /// `transpose((2, 1, 0))` before inference, so the tensor the network
    /// actually sees is a `(S, P, L)` view — the reverse axis order of the
    /// `(X, Y, Z)` canonical volume. Reorienting the raw buffer with this
    /// plan therefore yields exactly the trained-on layout in a single pass
    /// (no extra copy / transpose needed).
    ///
    /// This transform is an involution: `p.network_layout().network_layout()
    /// == p`, so it doubles as the inverse when mapping the predicted mask
    /// back to the original MHA `(Z, Y, X)` space.
    pub fn network_layout(&self) -> ReorientPlan {
        ReorientPlan {
            perm: [self.perm[2], self.perm[1], self.perm[0]],
            flip: [self.flip[2], self.flip[1], self.flip[0]],
            original_shape: self.original_shape,
        }
    }

    /// Spacing of the canonical volume, derived from the original
    /// `(Z,Y,X)` spacing (`spacing_zyx`) so axis `w` keeps the spacing of the
    /// old axis it was sourced from.
    pub fn canonical_spacing(&self, spacing_zyx: [f32; 3]) -> [f32; 3] {
        [
            spacing_zyx[self.perm[0]],
            spacing_zyx[self.perm[1]],
            spacing_zyx[self.perm[2]],
        ]
    }
}

/// Reorient a `Z,Y,X` row-major `f32` volume to canonical `(X,Y,Z)` (R,A,S)
/// layout. Returns the reoriented buffer and its `(X, Y, Z)` shape.
pub fn reorient_volume_to_canonical(
    data: &[f32],
    (zd, yh, xw): (usize, usize, usize),
    plan: &ReorientPlan,
) -> (Vec<f32>, (usize, usize, usize)) {
    let old = [zd, yh, xw];
    let nd = old[plan.perm[0]];
    let nh = old[plan.perm[1]];
    let nw = old[plan.perm[2]];
    let mut out = vec![0.0f32; nd * nh * nw];

    for nz in 0..nd {
        let so0 = if plan.flip[0] { old[plan.perm[0]] - 1 - nz } else { nz };
        for ny in 0..nh {
            let so1 = if plan.flip[1] { old[plan.perm[1]] - 1 - ny } else { ny };
            for nx in 0..nw {
                let so2 = if plan.flip[2] { old[plan.perm[2]] - 1 - nx } else { nx };
                // Map canonical (nz,ny,nx) back to original (Z,Y,X) indices.
                let mut co = [0usize; 3];
                co[plan.perm[0]] = so0;
                co[plan.perm[1]] = so1;
                co[plan.perm[2]] = so2;
                out[(nz * nh + ny) * nw + nx] = data[(co[0] * yh + co[1]) * xw + co[2]];
            }
        }
    }
    (out, (nd, nh, nw))
}

/// Inverse of [`reorient_volume_to_canonical`] for a `u8` label mask: move a
/// canonical `(X,Y,Z)` mask back to the original `Z,Y,X` layout so it overlays
/// the source MHA. `canonical_shape` must be `(old[perm[0]], old[perm[1]],
/// old[perm[2]])`.
pub fn reorient_mask_to_original(
    mask: &[u8],
    canonical_shape: (usize, usize, usize),
    plan: &ReorientPlan,
) -> Vec<u8> {
    let old = [
        plan.original_shape.0,
        plan.original_shape.1,
        plan.original_shape.2,
    ];
    let (_cd, ch, cw) = canonical_shape;
    let mut out = vec![0u8; old[0] * old[1] * old[2]];

    for zo in 0..old[0] {
        for yo in 0..old[1] {
            for xo in 0..old[2] {
                let orig = [zo, yo, xo];
                // Canonical coordinate for this original voxel.
                let mut cc = [0usize; 3];
                for w in 0..3 {
                    let a = plan.perm[w];
                    let coord = orig[a];
                    cc[w] = if plan.flip[w] { old[a] - 1 - coord } else { coord };
                }
                out[(zo * old[1] + yo) * old[2] + xo] =
                    mask[(cc[0] * ch + cc[1]) * cw + cc[2]];
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_orientation_matches_reference_reader() {
        // A typical TotalSegmentator MHA has `TransformMatrix = identity`,
        // i.e. `orientation = I` in ITK/LPS world. After the LPS->RAS
        // conversion the nibabel matrix is diag(-1,-1,1), which yields a
        // canonical plan of perm (2,1,0) / flips (T,T,F), and a network
        // layout of perm (0,1,2) / flips (F,T,T).
        //
        // Those two triples were confirmed EXACT against nnU-Net's own
        // `NibabelIOWithReorient` reader on a real spine CT (brute-forcing
        // all 48 signed permutations; the next best match differed by 1456
        // grey levels).
        let ori = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
        let plan = ReorientPlan::from_orientation(ori);
        assert_eq!(plan.perm, [2, 1, 0]);
        assert_eq!(plan.flip, [true, true, false]);

        let net = plan.network_layout();
        assert_eq!(net.perm, [0, 1, 2]);
        assert_eq!(net.flip, [false, true, true]);
    }

    #[test]
    fn network_layout_is_an_involution() {
        let ori = [[0.0, 1.0, 0.0], [0.0, 0.0, 1.0], [1.0, 0.0, 0.0]];
        let plan = ReorientPlan::from_orientation(ori);
        let round = plan.network_layout().network_layout();
        assert_eq!(round.perm, plan.perm);
        assert_eq!(round.flip, plan.flip);
    }

    #[test]
    fn identity_is_transpose_roundtrip() {
        // Forward then inverse must reproduce the input for any plan.
        let ori = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
        let mut plan = ReorientPlan::from_orientation(ori);
        plan.original_shape = (4, 5, 6);
        let vol: Vec<f32> = (0..(4 * 5 * 6)).map(|i| i as f32).collect();
        let (can, shp) = reorient_volume_to_canonical(&vol, (4, 5, 6), &plan);
        assert_eq!(shp, (6, 5, 4));
        let back = reorient_mask_to_original(
            &can.iter().map(|&v| v as u8).collect::<Vec<_>>(),
            shp,
            &plan,
        );
        assert_eq!(back, vol.iter().map(|&v| v as u8).collect::<Vec<_>>());
    }
}
