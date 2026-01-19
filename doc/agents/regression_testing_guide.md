# Regression Testing Guide

## Overview

Regression tests prevent bug recurrence by adding a test case for each bug fix. This guide defines the process for creating, naming, and maintaining regression tests in the Kepler2-WGPU medical imaging codebase.

## Regression Test Naming Convention

### Primary Format: `regression_issue_NNN_symptom`

**Pattern**: `regression_issue_<issue_number>_<symptom_description>`

**Examples**:
- `regression_issue_123_window_clamp_crash` - Issue #123: Crash when window level is clamped
- `regression_issue_456_dicom_uid_validation` - Issue #456: Invalid UID not rejected
- `regression_issue_789_texture_upload_bounds` - Issue #789: Texture upload exceeds bounds

### Fallback Format: `regression_module_symptom`

**Pattern**: `regression_module_<module>_<symptom_description>`

**Used when**: Bug doesn't have a GitHub issue number (e.g., discovered during development)

**Examples**:
- `regression_dicom_pixel_overflow` - Pixel value overflow in DICOM rescaling
- `regression_mpr_slice_precision` - MPR slice position precision loss
- `regression_texture_memory_leak` - Texture memory not freed

## Regression Test Template

### File: `tests/regression_tests.rs`

```rust
#[cfg(test)]
mod regression_tests {
    use crate::data::dicom::CTImage;
    use crate::data::ct_volume::CTVolume;
    use crate::rendering::view::mpr::MprView;

    /// Regression test for Issue #123
    ///
    /// Bug: Window level clamping caused crash when level exceeded 2048
    /// Fix: Add bounds checking and return error instead of panicking
    #[test]
    #[ignore] // TODO: Remove #[ignore] when bug is fixed
    fn regression_issue_123_window_clamp_crash() {
        let mut view = MprView::new();

        // This used to crash (panic: index out of bounds)
        // Now should return error gracefully
        let result = view.set_window_level(5000.0); // Exceeds MAX_WINDOW_LEVEL

        assert!(result.is_err());
        assert!(matches!(result, Err(MprError::InvalidWindowLevel(_))));
    }

    /// Regression test for Issue #456
    ///
    /// Bug: Invalid UID format was not rejected, allowing malformed DICOM files
    /// Fix: Add UID validation (only digits and periods, max 64 chars)
    #[test]
    #[ignore]
    fn regression_issue_456_dicom_uid_validation() {
        let invalid_uid = "1.2.840.113619.2.55.3.603610938272815658.20190101.120000.INVALID";

        let result = CTImage::validate_uid(invalid_uid);

        assert!(result.is_err());
        assert!(matches!(result, Err(DicomError::InvalidUidFormat(_))));
    }

    /// Regression test for texture memory leak
    ///
    /// Bug: Texture memory was not freed after view was destroyed
    /// Fix: Explicitly call texture destroy in MprView::drop
    #[test]
    #[ignore]
    fn regression_texture_memory_leak() {
        use std::alloc::{GlobalAlloc, Layout, System};

        let before_allocations = System.allocated_count();

        {
            let mut view = MprView::new();
            view.load_test_volume();
        } // View (and texture) should be dropped here

        let after_allocations = System.allocated_count();

        // Verify memory was freed
        assert_eq!(before_allocations, after_allocations);
    }

    /// Regression test for MPR slice precision loss
    ///
    /// Bug: Roundtrip coordinate transformation lost precision > 0.001 mm
    /// Fix: Use f64 for intermediate calculations, clamp to f32 at end
    #[test]
    #[ignore]
    fn regression_mpr_slice_precision() {
        let mut view = MprView::new();

        let world_coord = glam::Vec3::new(123.456, 789.012, 345.678);
        view.set_slice_position(world_coord);

        let screen_coord = view.world_to_screen(world_coord);
        let world_coord_back = view.screen_to_world(screen_coord);

        // Roundtrip error should be < 0.001 mm
        let error = (world_coord - world_coord_back).length();
        assert!(error < 0.001, "Roundtrip error: {}", error);
    }
}
```

## Adding Regression Test Workflow

### Step 1: Fix Bug

When fixing a bug:
1. Identify root cause
2. Implement fix
3. Write test that reproduces the bug

### Step 2: Create Regression Test

Follow this template:

```rust
/// Regression test for Issue #NNN
///
/// Bug: [Description of bug]
/// Fix: [Description of fix]
#[test]
#[ignore] // Keep ignored until fix is verified
fn regression_issue_NNN_symptom() {
    // 1. Setup: Create conditions that trigger bug
    // 2. Action: Call function that used to fail
    // 3. Assertion: Verify bug is fixed

    // Example:
    let result = function_that_used_to_fail();
    assert!(result.is_ok()); // Bug is fixed
}
```

### Step 3: Verify Fix

1. Remove `#[ignore]` attribute
2. Run test to verify it passes
3. Commit test with bug fix

### Step 4: Document Test

Update test metadata in regression test comment:
```rust
/// Regression test for Issue #NNN
///
/// Bug: [Description]
/// Fix: [Description]
/// Issue: https://github.com/user/repo/issues/NNN
/// Date added: YYYY-MM-DD
/// Fix commit: <commit-hash>
```

## Regression Test Tracking

### Regression Test Register

**File**: `tests/regression_tests.md`

```markdown
# Regression Test Register

| Issue # | Test Name | Symptom | Date Added | Fix Commit | Status |
|----------|-----------|----------|-------------|-------------|---------|
| #123 | regression_issue_123_window_clamp_crash | Crash on window level clamp | 2024-01-15 | abc123def | Fixed |
| #456 | regression_issue_456_dicom_uid_validation | Invalid UID not rejected | 2024-01-16 | def456ghi | Fixed |
| - | regression_texture_memory_leak | Texture memory leak | 2024-01-17 | ghi789jkl | In Progress |
```

### Tracking Script

**File**: `scripts/update_regression_tests.sh`

```bash
#!/bin/bash
# Update regression test register from code

echo "# Regression Test Register" > tests/regression_tests.md
echo "" >> tests/regression_tests.md
echo "| Issue # | Test Name | Symptom | Date Added | Fix Commit | Status |" >> tests/regression_tests.md
echo "|----------|-----------|----------|-------------|-------------|---------|" >> tests/regression_tests.md

# Parse regression test comments
rg "/// Regression test for Issue #" tests/regression_tests.rs \
    | sed 's/.*Issue #\([0-9]*\).*/\1/' \
    | while read issue; do

    test_name=$(rg "fn regression_issue_${issue}_" tests/regression_tests.rs | sed 's/.*fn \([^(]*\).*/\1/')
    echo "| #$issue | $test_name | TODO | $(date +%Y-%m-%d) | TODO | New |" >> tests/regression_tests.md
done
```

## Regression Test Execution

### Running All Regression Tests

```bash
# Run only regression tests
cargo test regression

# Run all tests including regression
cargo test

# Run specific regression test
cargo test regression_issue_123
```

### CI Integration

```yaml
# .github/workflows/regression-tests.yml
name: Regression Tests

on: [push, pull_request]

jobs:
  regression:
    runs-on: ubuntu-latest

    steps:
      - uses: actions/checkout@v3

      - name: Run regression tests
        run: cargo test regression

      - name: Fail if regression test fails
        run: |
          if cargo test regression 2>&1 | grep -q "FAILED"; then
            echo "ERROR: Regression test failed! Bug may have reappeared."
            exit 1
          fi
```

## Regression Test Best Practices

### 1. Test Is Minimal Reproducer

**Bad**: Test with 1000 lines of setup code

```rust
#[test]
fn regression_issue_123_window_clamp_crash() {
    // 100 lines of DICOM parsing
    // 100 lines of volume creation
    // 100 lines of texture upload
    // 100 lines of MPR view setup
    // ...
    view.set_window_level(5000.0); // Actual bug is here
    // ... more assertions
}
```

**Good**: Test with minimal setup

```rust
#[test]
fn regression_issue_123_window_clamp_crash() {
    let mut view = MprView::new(); // Minimal setup
    view.set_window_level(5000.0); // Directly test bug
    assert!(view.window_level().is_err()); // Verify fix
}
```

### 2. Use Meaningful Assertions

**Bad**: Generic assertion

```rust
assert!(result.is_ok()); // Doesn't tell what was wrong
```

**Good**: Specific assertion with error message

```rust
assert!(
    result.is_ok(),
    "Expected success but got error: {:?}",
    result.err()
);
```

### 3. Reference Original Bug Report

```rust
/// Regression test for Issue #123
///
/// Bug: Application crashes when window level is set to 5000.0
/// Original bug report: https://github.com/user/repo/issues/123
/// Steps to reproduce:
/// 1. Load patient scan
/// 2. Set window level to 5000
/// 3. Application panics with "index out of bounds"
/// Fix: Add bounds checking and return MprError::InvalidWindowLevel
```

### 4. Keep Test Focused on Bug

**Bad**: Test covers multiple unrelated bugs

```rust
#[test]
fn regression_issue_123_window_clamp_crash() {
    view.set_window_level(5000.0); // Bug #123
    assert!(result.is_ok());

    view.set_pan(100000.0); // Bug #456
    assert!(pan_result.is_ok());

    view.load_volume(volume); // Bug #789
    assert!(load_result.is_ok());
}
```

**Good**: One test per bug

```rust
#[test]
fn regression_issue_123_window_clamp_crash() {
    view.set_window_level(5000.0);
    assert!(result.is_ok());
}

#[test]
fn regression_issue_456_pan_bounds() {
    view.set_pan(100000.0);
    assert!(pan_result.is_ok());
}

#[test]
fn regression_issue_789_volume_load() {
    view.load_volume(volume);
    assert!(load_result.is_ok());
}
```

## Removing Old Regression Tests

### When to Remove

Regression tests can be removed when:
1. The code they test is completely removed/refactored
2. The test has been passing for 6+ months without issue
3. The test is redundant with other comprehensive tests

### Removal Process

1. **Document removal**: Add comment explaining why test is removed
2. **Verify coverage**: Ensure removal doesn't reduce test coverage
3. **Archive test**: Move to `tests/archive/` directory instead of deleting
4. **Update register**: Remove from `tests/regression_tests.md`

```rust
// REMOVED: regression_issue_123_window_clamp_crash
// Reason: Window level clamping is now handled by MprView::validate_inputs(),
// which has comprehensive tests. This regression test is redundant.
// Removed: 2024-06-15
// Archive location: tests/archive/2024/regression_issue_123.rs
```

## PR Guidelines: Require Regression Tests

### PR Template

```markdown
## Bug Fix

- [ ] Fixes issue #[issue-number]
- [ ] Added regression test: `regression_issue_XXX_symptom`
- [ ] Regression test passes
- [ ] Updated regression test register
```

### Automated Check

**File**: `.github/workflows/pr-check.yml`

```yaml
name: PR Regression Test Check

on:
  pull_request:
    types: [opened, edited, synchronize]

jobs:
  check_regression_tests:
    runs-on: ubuntu-latest

    steps:
      - uses: actions/checkout@v3

      - name: Check for regression test
        run: |
          if rg "#[0-9]+" PR_description.md | grep -q "fixes"; then
            if ! rg "regression_issue_" tests/regression_tests.rs | grep -q "fn regression_issue_${issue}"; then
              echo "ERROR: PR fixes bug #${issue} but has no regression test"
              exit 1
            fi
          fi
```

## References

- Regression testing definition: https://en.wikipedia.org/wiki/Regression_testing
- Rust testing patterns: https://doc.rust-lang.org/book/ch11-00-testing.html
- Test naming conventions: https://testing.googleblog.com/2015/08/the-google-testing-blog.html
