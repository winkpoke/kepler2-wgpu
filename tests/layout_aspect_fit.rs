#![allow(clippy::float_cmp)]

use kepler_wgpu::rendering::view::compute_aspect_fit;

fn approx_eq(a: f32, b: f32, eps: f32) -> bool {
    (a - b).abs() <= eps
}

#[test]
fn fit_square_in_square_no_padding() {
    let res = compute_aspect_fit(400, 400, 100.0, 100.0, 0).unwrap();
    assert!(approx_eq(res.x, 0.0, 1e-3));
    assert!(approx_eq(res.y, 0.0, 1e-3));
    assert!(approx_eq(res.w, 400.0, 1e-3));
    assert!(approx_eq(res.h, 400.0, 1e-3));
}

#[test]
fn fit_16x9_in_4x3_letterbox() {
    let res = compute_aspect_fit(400, 300, 16.0, 9.0, 0).unwrap();
    // width matches container, height reduced (letterbox)
    assert!(approx_eq(res.w, 400.0, 1e-2));
    assert!(approx_eq(res.h, 400.0 / (16.0 / 9.0), 1e-2));
    // centered vertically
    assert!(approx_eq(res.x, 0.0, 1e-2));
    assert!(approx_eq(res.y, (300.0 - res.h) * 0.5, 1e-3));
}

#[test]
fn fit_4x3_in_16x9_pillarbox() {
    let res = compute_aspect_fit(1600, 900, 4.0, 3.0, 0).unwrap();
    // height matches container, width reduced (pillarbox)
    assert!(approx_eq(res.h, 900.0, 1e-2));
    assert!(approx_eq(res.w, 900.0 * (4.0 / 3.0), 1e-2));
    // centered horizontally
    assert!(approx_eq(res.y, 0.0, 1e-3));
    assert!(approx_eq(res.x, (1600.0 - res.w) * 0.5, 1e-2));
}

#[test]
fn fit_with_padding_clamped() {
    // padding larger than half min(container) is clamped internally
    let res = compute_aspect_fit(100, 50, 1.0, 1.0, 100).unwrap();
    assert!(res.w > 0.0 && res.h > 0.0);
}

#[test]
fn invalid_inputs_return_none() {
    assert!(compute_aspect_fit(0, 100, 1.0, 1.0, 0).is_none());
    assert!(compute_aspect_fit(100, 0, 1.0, 1.0, 0).is_none());
    assert!(compute_aspect_fit(100, 100, 0.0, 1.0, 0).is_none());
    assert!(compute_aspect_fit(100, 100, 1.0, 0.0, 0).is_none());
}
