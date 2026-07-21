// Needle uniform
struct NeedleUniform {
    entry: vec3<f32>,
    radius: f32,
    tip: vec3<f32>,
    id: u32,
    color: vec4<f32>,
};

// Returns true if `p` lies inside a finite cylinder between `entry` and `tip`
// with the given `radius`. Treats degenerate (entry≈tip) needles as empty.
fn point_inside_needle(p: vec3<f32>, entry: vec3<f32>, tip: vec3<f32>, radius: f32) -> bool {
    let axis = tip - entry;
    let len = length(axis);
    if (len < 0.00001) {
        return false;
    }
    let dir = axis / len;
    let v = p - entry;
    let t = dot(v, dir);
    if (t < 0.0 || t > len) {
        return false;
    }
    let closest = entry + dir * t;
    let dist = distance(p, closest);
    return dist < radius;
}

// Returns true if `p` lies inside a finite-thickness disc centred at `center`,
// with face-normal `axis` and radius `radius`. Thickness is measured as
// `thickness` along `axis` (i.e. the disc is a cylinder cap with `thickness` height).
fn point_inside_disc(p: vec3<f32>, center: vec3<f32>, axis: vec3<f32>, radius: f32, thickness: f32) -> bool {
    let v = p - center;
    let h = dot(v, axis);
    if (abs(h) > thickness * 0.5) {
        return false;
    }
    let radial = v - h * axis;
    return dot(radial, radial) <= radius * radius;
}