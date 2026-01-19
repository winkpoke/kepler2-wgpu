# Coverage Calculation Methodology

## Overview

This document defines how test coverage is calculated, what is included/excluded, what tools are used, and what thresholds mean for the Kepler2-WGPU medical imaging codebase.

## Coverage Type: Branch Coverage

**Decision**: Use **branch coverage** (not just line coverage)

**Rationale**:
- Branch coverage is more meaningful for medical safety-critical code
- Line coverage can hide untested conditional branches
- Example: `if medical_condition { treat() }` - line covered but `else` path not tested
- Branch coverage ensures both code paths are tested

### Line vs Branch Coverage Example

```rust
// Line coverage: 100% (line 2 executed)
// Branch coverage: 50% (only `true` path tested)

fn validate_pixel_value(pixel: i16) -> Result<i16, Error> {
    if pixel < MIN_PIXEL_VALUE {              // Line 2
        return Err(Error::InvalidPixel);       // This path not tested!
    }
    Ok(pixel)                               // Line 4
}
```

## Coverage Tool: cargo-llvm-cov

**Chosen Tool**: `cargo-llvm-cov` (industry standard for Rust)

### Installation

```bash
# Add to dev-dependencies in Cargo.toml
[dev-dependencies]
cargo-llvm-cov = "0.5"

# Install tool
cargo install cargo-llvm-cov
```

### Configuration

**File**: `.cargo/config.toml`

```toml
[llvm-cov]
# Generate HTML reports
html = true

# Output directory
output-dir = "coverage"

# Include only source code, not tests
exclude-lines = [
    "mod tests",
    "#\[cfg(test)\]",
]
```

### Usage

```bash
# Generate coverage report
cargo llvm-cov --html --output-dir coverage

# Generate coverage for specific test
cargo llvm-cov --html -- test dicom_validation

# Generate coverage with lcov format (for CI)
cargo llvm-cov --lcov --output-dir coverage
```

## Coverage Calculation Formula

### Calculation Method

```
coverage_percent = (covered_branches / total_branches) * 100
```

Where:
- `covered_branches`: Number of conditional branches executed in tests
- `total_branches`: Total number of conditional branches in code

### What Counts as a Branch?

1. **If/Else Statements**: Each `if` creates 2 branches
2. **Match Statements**: Each match arm is a branch
3. **Boolean Expressions**: `&&` and `||` create branches
4. **Early Returns**: Each `return` creates a branch
5. **Error Propagation**: `?` operator creates success/failure branches

### Excluded Code

**Branches in excluded code are NOT counted**:
- Unsafe blocks (unverified by test coverage)
- FFI calls (external code, cannot test)
- Platform-specific code (`#[cfg(target_arch = "wasm32")]` or `#[cfg(not(target_arch = "wasm32"))]`)
- Generated code (derived implementations, macros)
- Test code itself

## Medical Path Coverage Definition

### Medical Paths: What's Included

Medical paths are code that directly affects patient safety:

1. **DICOM Parsing Code**
   - Directory: `src/data/dicom/`
   - Files: `ct_image.rs`, `patient.rs`, `studyset.rs`, `fileio.rs`
   - Why: Validates mandatory fields, ensures data integrity

2. **Patient Metadata Code**
   - Directory: `src/data/dicom/`
   - Files: `patient.rs`, `studyset.rs`
   - Why: Patient identity, study/series integrity

3. **Coordinate Transformation Code**
   - Directory: `src/rendering/view/mpr/`
   - Files: `mpr_view.rs`, `mpr_view_wgpu_impl.rs`
   - Why: Anatomical accuracy, prevents distortion

4. **Window/Level Transformations**
   - Directory: `src/core/`
   - Files: `window_level.rs`
   - Why: Hounsfield unit display, diagnostic accuracy

### Medical Path Coverage Target: ≥ 80%

**Rationale**:
- Patient safety requires high confidence in critical paths
- 80% coverage means 4 out of 5 code paths tested
- Higher than overall target (45-70%) because medical paths are safety-critical

### Example: Medical Path Coverage Report

```
Medical Path Coverage: 82%

Medical Paths:
- src/data/dicom/ct_image.rs:          85% (17/20 branches)
- src/data/dicom/patient.rs:           78% (18/23 branches)
- src/data/dicom/studyset.rs:          80% (12/15 branches)
- src/rendering/view/mpr/mpr_view.rs:    83% (15/18 branches)
- src/core/window_level.rs:              85% (11/13 branches)

Target: ≥ 80% ✓
```

## Exclusion Policy

### 1. Unsafe Blocks

**Reason**: Unsafe code bypasses Rust's memory safety guarantees and cannot be reliably tested with coverage.

```rust
unsafe {
    // Direct memory access, FFI calls, raw pointers
    // Exclude from coverage calculation
}
```

**Action**:
- Add `#[cfg_attr(coverage, no_coverage)]` to unsafe functions
- Document unsafe blocks with safety invariants
- Require manual code review instead of coverage

### 2. FFI Calls (Foreign Function Interface)

**Reason**: Cannot test external library code (C/C++ dependencies, OpenGL, Vulkan, WebGPU).

```rust
extern "C" {
    fn external_function(); // Cannot test
}
```

**Action**:
- Exclude FFI wrapper functions from coverage
- Test FFI behavior through integration tests (test the wrapper, not the external code)

### 3. Platform-Specific Code

**Reason**: Cannot test WASM code on native platform and vice-versa.

```rust
#[cfg(target_arch = "wasm32")]
fn wasm_specific_function() {
    // Cannot test on native (cargo test runs native by default)
}

#[cfg(not(target_arch = "wasm32"))]
fn native_specific_function() {
    // Cannot test on WASM
}
```

**Action**:
- Exclude platform-specific code when running on opposite platform
- Use conditional compilation in CI to measure coverage on both platforms

### 4. Generated Code

**Reason**: Derived implementations, procedural macros, and code generated by macros are not human-written.

```rust
#[derive(Debug, Clone, PartialEq)]
struct Data {
    // Auto-generated Debug, Clone, PartialEq implementations
}
```

**Action**:
- Exclude `#[derive]` implementations from coverage
- Focus coverage on business logic, not boilerplate

### 5. Test Code

**Reason**: Test code should not count toward coverage of production code.

```rust
#[cfg(test)]
mod tests {
    #[test]
    fn test_something() {
        // This code is not production code
    }
}
```

**Action**:
- Exclude all `#[cfg(test)]` modules and functions
- Only count coverage in `src/` directory

## Threshold Meanings

### Overall Coverage Thresholds

| Phase | Target | Meaning | Safety Impact |
|-------|--------|---------|---------------|
| Phase 1 (Week 2) | 45% | 45% of branches tested | Baseline for medical safety paths (80%) |
| Phase 2 (Week 4) | 60% | 60% of branches tested | Improved data integrity coverage |
| Phase 3 (Week 6) | 65% | 65% of branches tested | Rendering correctness coverage |
| Phase 4 (Week 8) | 70% | 70% of branches tested | Error handling and robustness |
| Phase 5 (Ongoing) | 80%+ on critical paths | 80% of medical path branches tested | Ongoing patient safety maintenance |

### Medical Path Threshold: 80%

**Meaning**: For every 5 code branches in medical path code, at least 4 are tested.

**Risk Assessment**:
- 80% coverage: Low risk (4/5 paths tested)
- 50% coverage: Medium risk (1/2 paths untested)
- < 50% coverage: High risk (more paths untested than tested)

**Medical Safety Implications**:
- Untested branch → potential patient data corruption
- Untested error path → crash during patient scan loading
- Untested validation → malformed DICOM data reaches rendering

### Example: 75% vs 85% Coverage

```rust
fn validate_dicom_tag(tag: DicomTag) -> Result<(), Error> {
    if tag.group % 2 == 0 {          // Branch 1
        if tag.element == 0x0010 {     // Branch 2
            return Ok(());               // Path 1 (tested)
        } else if tag.element == 0x0020 {  // Branch 3
            return Ok(());               // Path 2 (tested)
        }
        return Err(Error::InvalidTag);    // Path 3 (tested)
    }
    return Err(Error::InvalidTagGroup);  // Path 4 (untested!)
}

// 75% coverage: 3 paths tested, 1 untested (group odd)
// 85% coverage: 5 paths tested, 0 untested (add test for odd group)
```

## Coverage Reporting

### HTML Report (Local Development)

```bash
cargo llvm-cov --html --output-dir coverage
open coverage/index.html
```

**Features**:
- Color-coded files: Green (high coverage), Red (low coverage), Yellow (partial)
- Click file to see line-by-line coverage
- Hover over lines to see execution count

### CI Coverage Report (GitHub Actions)

**File**: `.github/workflows/coverage.yml`

```yaml
name: Coverage Report

on: [push, pull_request]

jobs:
  coverage:
    runs-on: ubuntu-latest

    steps:
      - uses: actions/checkout@v3

      - name: Install Rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
          override: true

      - name: Generate coverage report
        run: |
          cargo install cargo-llvm-cov
          cargo llvm-cov --lcov --output-dir coverage

      - name: Upload to Codecov
        uses: codecov/codecov-action@v3
        with:
          file: ./coverage/lcov.info
          flags: unittests
          name: codecov-umbrella

      - name: Coverage trend detection
        run: |
          # Compare with previous coverage
          # Fail if coverage decreases > 5%
          ./scripts/check_coverage_regression.sh
```

### Coverage Comment in PR

Use **codecov-comment-action** to add coverage summary to PR:

```yaml
- name: Comment coverage in PR
  uses: romeovs/lcov-reporter-action@v0.3.1
  with:
    lcov-file: ./coverage/lcov.info
    github-token: ${{ secrets.GITHUB_TOKEN }}
```

**Output**: Coverage comment in PR shows:
- Overall coverage percentage
- Changed files coverage
- Missing branches in changed files

## Coverage Trends

### Regression Detection

**Threshold**: Fail if coverage decreases > 5% from baseline

**Implementation**:
```bash
#!/bin/bash
# scripts/check_coverage_regression.sh

CURRENT=$(cargo llvm-cov --quiet | grep -oP '\d+(?=\%)')
BASELINE=$(git show HEAD~1:coverage_report.txt | grep -oP '\d+(?=\%)')

DECREASE=$(echo "$BASELINE - $CURRENT" | bc)

if (( $(echo "$DECREASE > 5" | bc -l) )); then
    echo "ERROR: Coverage decreased by $DECREASE% (baseline: $BASELINE%, current: $CURRENT%)"
    exit 1
fi
```

## Adjusting Targets Based on Exclusions

### Example Calculation

**Scenario**: 20% of code is excluded (unsafe, FFI, platform-specific)

```rust
// Total branches: 100
// Excluded branches: 20
// Remaining branches: 80

// Target: 70% overall coverage
// Required covered branches: 70% * 80 = 56 branches

// Adjusted threshold for remaining code:
// 56 / 80 = 70% (unchanged if exclusions are consistent)
```

**Policy**:
- Coverage targets apply to **testable code only**
- Excluded code percentages are tracked separately
- If exclusions > 30%, review if coverage target is still meaningful

## References

- Cargo llvm-cov documentation: https://github.com/taiki-e/cargo-llvm-cov
- Codecov integration: https://docs.codecov.com/docs
- Branch coverage explanation: https://en.wikipedia.org/wiki/Code_coverage
- Medical device testing standards: IEC 62304 (Medical device software - Life cycle processes)
