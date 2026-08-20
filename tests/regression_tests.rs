//! Regression tests
//!
//! This module provides regression tests for known bugs and issues.
//!
//! Naming convention:
//! - `regression_issue_NNN_symptom` - Tests for bugs tracked in GitHub issues
//! - `regression_module_symptom` - Tests for bugs without GitHub issue numbers
//!
//! Guidelines:
//! - Include issue link in docstring
//! - Mark with `#[ignore]` until bug is fixed
//! - Remove `#[ignore]` when bug is fixed and test passes

#[cfg(test)]
mod regression_template_tests {
    use super::*;

    /// Example regression test template
    ///
    /// This is a template for adding new regression tests.
    /// Copy and modify this template when adding new regression tests.
    ///
    /// # Template Structure
    ///
    /// 1. Include issue reference in docstring
    /// 2. Mark `#[ignore]` if bug is not yet fixed
    /// 3. Write test that reproduces the bug
    /// 4. Assert expected behavior when bug is fixed
    #[test]
    #[ignore]
    fn regression_template() {}
}

#[cfg(test)]
mod regression_example_tests {
    use super::*;

    /// Regression test for example issue
    ///
    /// This is an example regression test. Replace this with actual issue details
    /// when creating a new regression test for a real bug.
    ///
    /// Issue: https://github.com/user/repo/issues/123
    /// Symptom: Description of the bug/symptom
    #[test]
    #[ignore]
    fn regression_issue_123_example_symptom() {}
}

/// Regression test: `ServerState::set_ptm` must persist across clones.
///
/// Axum's `State<ServerState>` extractor hands every request a clone of the
/// router-held state. `ptm` used to be stored by value, so `set_ptm` in
/// `/api/needle_point_offset` only mutated the per-request clone and
/// `/api/upload_needle_params` still read `Mat4::IDENTITY`, dropping the
/// patient transform on entry/tip coordinates.
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn regression_server_state_ptm_persists_across_clones() {
    use glam::{Mat4, Vec4};

    let router_state = kepler_wgpu::server::state::ServerState::new();
    // Simulate a second request: axum clones the state for the handler.
    let handler_state = router_state.clone();

    let ptm = Mat4::from_cols(
        Vec4::new(0.999950882, -0.007056614, -0.006959693, 0.0),
        Vec4::new(0.007074652, 0.999971670, 0.002570663, 0.0),
        Vec4::new(0.006941356, -0.002619774, 0.999972477, 0.0),
        Vec4::new(-5.427908, 87.284000, -582.195858, 1.0),
    );
    handler_state.set_ptm(ptm);

    assert_eq!(router_state.get_ptm(), ptm);

    // A third request (another clone) must observe the same matrix.
    assert_eq!(router_state.clone().get_ptm(), ptm);
}
