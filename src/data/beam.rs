//! Patient-space geometric beam intersections. No UI or GPU ownership.
use crate::{
    core::{KeplerError, KeplerResult},
    data::{bev::BeamProjection, slice::SlicePlane},
};
use glam::{DVec2, DVec3};
use std::sync::Arc;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ApertureEdge {
    pub a: DVec2,
    pub b: DVec2,
}

/// Explicit conventional beam geometry in patient LPS and isocenter-plane mm.
#[derive(Clone, Debug)]
pub struct BeamOverlayGeometry {
    pub isocenter: DVec3,
    pub x: DVec3,
    pub y: DVec3,
    pub z: DVec3,
    pub sad: f64,
    pub jaws: Option<[f64; 4]>,
    pub aperture: Option<Arc<[ApertureEdge]>>,
}
impl BeamOverlayGeometry {
    pub fn validate(&self) -> KeplerResult<()> {
        BeamProjection::new(self.isocenter, self.x, self.y, self.z, self.sad)?;
        if self
            .jaws
            .is_some_and(|j| !j.iter().all(|v| v.is_finite()) || j[0] > j[1] || j[2] > j[3])
            || self.aperture.as_ref().is_some_and(|a| {
                a.len() > 2048 || a.iter().any(|e| !e.a.is_finite() || !e.b.is_finite())
            })
        {
            return Err(KeplerError::Validation(
                "Invalid beam aperture geometry".into(),
            ));
        }
        Ok(())
    }
}
/// Intersect aperture-edge ray surfaces with the physical MPR plane. Returns full-image UV
/// segments, clipped to the finite edge rays and the displayed rectangle. No voxels are read.
pub fn slice_segments(beam: &BeamOverlayGeometry, plane: &SlicePlane) -> Vec<[DVec2; 2]> {
    let Some(aperture) = &beam.aperture else {
        return Vec::new();
    };
    let origin = plane.origin - beam.isocenter;
    let right = plane.right * plane.extent_mm[0];
    let down = plane.down * plane.extent_mm[1];
    let source = beam.z * beam.sad;
    let mut result = Vec::with_capacity(aperture.len());
    for segment in aperture.iter() {
        let ray_a = beam.x * segment.a.x + beam.y * segment.a.y - source;
        let ray_b = beam.x * segment.b.x + beam.y * segment.b.y - source;
        let cross = ray_a.cross(ray_b);
        let determinant = cross.length_squared();
        if determinant <= 1e-16 {
            continue;
        }
        let normal = cross / determinant.sqrt();
        let coefficients = [
            (origin - source).dot(normal),
            right.dot(normal),
            down.dot(normal),
        ];
        let Some(mut line) = rectangle_line(coefficients) else {
            continue;
        };
        let values = line.map(|uv| {
            let p = origin + right * uv.x + down * uv.y - source;
            [
                p.cross(ray_b).dot(cross) / determinant,
                ray_a.cross(p).dot(cross) / determinant,
            ]
        });
        let mut limits = [0., 1.];
        if !(0..2).all(|i| clip_positive(values[0][i], values[1][i], &mut limits)) {
            continue;
        }
        let start = line[0];
        let delta = line[1] - start;
        line = [start + delta * limits[0], start + delta * limits[1]];
        if line[0].distance_squared(line[1]) > 1e-20 {
            result.push(line);
        }
    }
    result
}

fn rectangle_line([offset, x, y]: [f64; 3]) -> Option<[DVec2; 2]> {
    let mut hits = Vec::with_capacity(4);
    for edge in [0., 1.] {
        if y.abs() > 1e-10 {
            let v = -(offset + x * edge) / y;
            if (-1e-10..=1. + 1e-10).contains(&v) {
                hits.push(DVec2::new(edge, v.clamp(0., 1.)));
            }
        }
        if x.abs() > 1e-10 {
            let u = -(offset + y * edge) / x;
            if (-1e-10..=1. + 1e-10).contains(&u) {
                hits.push(DVec2::new(u.clamp(0., 1.), edge));
            }
        }
    }
    let mut longest = None;
    let mut distance = 1e-20;
    for a in &hits {
        for b in &hits {
            let d = a.distance_squared(*b);
            if d > distance {
                distance = d;
                longest = Some([*a, *b]);
            }
        }
    }
    longest
}

fn clip_positive(start: f64, end: f64, limits: &mut [f64; 2]) -> bool {
    let start = start + 1e-10;
    let end = end + 1e-10;
    if start < 0. && end < 0. {
        return false;
    }
    if start < 0. {
        limits[0] = limits[0].max(start / (start - end));
    }
    if end < 0. {
        limits[1] = limits[1].min(start / (start - end));
    }
    limits[0] <= limits[1]
}

/// Exact geometric union in one fixed isocenter frame. Input is bounded to 8192
/// recorded outlines, 2048 edges each and one million aggregate edges. Work and
/// output caps return explicit errors; no partial or rasterized envelope is returned.
pub fn union_axis_aligned_outlines(
    outlines: &[Arc<[ApertureEdge]>],
) -> KeplerResult<Arc<[ApertureEdge]>> {
    const MAX_WORK: usize = 4_000_000;
    const MAX_EDGES: usize = 16_384;
    if outlines.len() > 8192
        || outlines.iter().any(|a| a.len() > 2048)
        || outlines.iter().map(|a| a.len()).sum::<usize>() > 1_000_000
    {
        return Err(KeplerError::Validation(
            "Recorded envelope input limit exceeded".into(),
        ));
    }
    if outlines
        .iter()
        .flat_map(|a| a.iter())
        .any(|e| !e.a.is_finite() || !e.b.is_finite() || e.a.x != e.b.x && e.a.y != e.b.y)
    {
        return Err(KeplerError::Validation(
            "Envelope needs finite axis-aligned edges".into(),
        ));
    }
    let mut breaks: Vec<_> = outlines
        .iter()
        .flat_map(|a| a.iter().flat_map(|e| [e.a.y, e.b.y]))
        .collect();
    breaks.sort_by(f64::total_cmp);
    breaks.dedup();
    let edge_count: usize = outlines.iter().map(|a| a.len()).sum();
    if edge_count.saturating_mul(breaks.len()) > MAX_WORK {
        return Err(KeplerError::Validation(
            "Recorded envelope exceeds the bounded geometry work limit".into(),
        ));
    }
    let mut output = Vec::new();
    let mut previous: Vec<[f64; 2]> = Vec::new();
    for row in breaks.windows(2) {
        let y = row[0].midpoint(row[1]);
        let mut intervals = Vec::new();
        for outline in outlines {
            let mut xs: Vec<_> = outline
                .iter()
                .filter(|s| s.a.y.min(s.b.y) < y && s.a.y.max(s.b.y) > y)
                .map(|s| s.a.x)
                .collect();
            xs.sort_by(f64::total_cmp);
            if !xs.as_chunks::<2>().1.is_empty() {
                return Err(KeplerError::Validation("Open envelope boundary".into()));
            }
            for pair in xs.as_chunks::<2>().0 {
                if pair[0] < pair[1] {
                    intervals.push([pair[0], pair[1]]);
                }
            }
        }
        intervals.sort_by(|a, b| a[0].total_cmp(&b[0]));
        let mut current: Vec<[f64; 2]> = Vec::new();
        for interval in intervals {
            if let Some(last) = current.last_mut().filter(|p| p[1] >= interval[0]) {
                last[1] = last[1].max(interval[1]);
            } else {
                current.push(interval);
            }
        }
        horizontal_edges(&mut output, &previous, &current, row[0]);
        for interval in &current {
            for &x in interval {
                output.push(ApertureEdge {
                    a: DVec2::new(x, row[0]),
                    b: DVec2::new(x, row[1]),
                });
            }
        }
        if output.len() > MAX_EDGES {
            return Err(KeplerError::Validation(
                "Recorded envelope exceeds 16384 edges".into(),
            ));
        }
        previous = current;
    }
    if let Some(&y) = breaks.last() {
        horizontal_edges(&mut output, &previous, &[], y);
    }
    if output.len() > MAX_EDGES {
        return Err(KeplerError::Validation(
            "Recorded envelope exceeds 16384 edges".into(),
        ));
    }
    Ok(output.into())
}

fn horizontal_edges(output: &mut Vec<ApertureEdge>, a: &[[f64; 2]], b: &[[f64; 2]], y: f64) {
    for (left, right) in [(a, b), (b, a)] {
        for interval in left {
            let mut x = interval[0];
            for r in right
                .iter()
                .filter(|r| r[1] > interval[0] && r[0] < interval[1])
            {
                if r[0] > x {
                    output.push(ApertureEdge {
                        a: DVec2::new(x, y),
                        b: DVec2::new(r[0], y),
                    });
                }
                x = x.max(r[1]);
            }
            if x < interval[1] {
                output.push(ApertureEdge {
                    a: DVec2::new(x, y),
                    b: DVec2::new(interval[1], y),
                });
            }
        }
    }
}
