# Property-Based Testing Strategy

## Overview

Property-based testing uses `proptest` to test mathematical and geometric operations with randomly generated inputs to discover edge cases and verify invariants that may not be covered by example-based tests.

## Window/Level Property Tests

### Properties

1. **Monotonicity Property**: Increasing window level increases displayed brightness
   - **Input**: window_width ∈ [1.0, 4096.0], window_level ∈ [-2048.0, 2048.0], bias ∈ [-2048.0, 2048.0]
   - **Invariant**: If level1 < level2, then output(level1) < output(level2) for same pixel value
   - **Strategy**: `prop::num::f32::NORMAL..4096.0` for width, `prop::num::f32::NEG_INFINITY..2048.0` for level

2. **Invertibility Property**: Window level + bias transformation is reversible
   - **Input**: pixel_value ∈ [i16::MIN, i16::MAX], width ∈ [1.0, 4096.0], level ∈ [-2048.0, 2048.0]
   - **Invariant**: apply_window_level(pixel_value, width, level) → normalized → reverse_normalize(pixel_value, width, level) ≈ pixel_value
   - **Tolerance**: ±1 Hounsfield unit for floating point precision

3. **Bounds Preservation Property**: Effective level always in [MIN_WINDOW_LEVEL, MAX_WINDOW_LEVEL]
   - **Input**: level ∈ [f32::MIN, f32::MAX]
   - **Invariant**: effective_level = clamp(level, MIN_WINDOW_LEVEL, MAX_WINDOW_LEVEL)
   - **Constant**: MIN_WINDOW_LEVEL = -2048.0, MAX_WINDOW_LEVEL = 2048.0

4. **Range Preservation Property**: Output values always in [0.0, 1.0] after normalization
   - **Input**: pixel ∈ [i16::MIN, i16::MAX], width ∈ [1.0, 4096.0], level ∈ [-2048.0, 2048.0]
   - **Invariant**: 0.0 ≤ normalized_value ≤ 1.0
   - **Strategy**: `prop::num::i16::ANY` for pixel values

## Coordinate Transformation Property Tests

### Properties

1. **Matrix Determinant Property**: det(rotation) = 1.0 ± epsilon
   - **Input**: Rotation angles ∈ [0.0, 2π] for each axis
   - **Invariant**: Determinant of rotation matrix equals 1.0 (volume preserved)
   - **Epsilon**: 1e-6 for floating point tolerance
   - **Strategy**: Generate random Euler angles, construct rotation matrix, compute determinant

2. **Scale Clamping Property**: Scale always in [MIN_SCALE, MAX_SCALE]
   - **Input**: scale ∈ [0.001, 1000.0]
   - **Invariant**: clamped_scale = clamp(scale, MIN_SCALE, MAX_SCALE) = clamp(scale, 0.01, 100.0)
   - **Strategy**: `prop::num::f32::ANY` for scale input

3. **Pan Distance Property**: Pan always clamped to ±MAX_PAN_DISTANCE
   - **Input**: pan_distance ∈ [-50000.0, 50000.0]
   - **Invariant**: clamped_pan = clamp(pan_distance, -10000.0, 10000.0)
   - **Strategy**: `prop::num::f32::NEG_INFINITY..50000.0` for pan input

4. **Aspect Ratio Preservation Property**: Aspect fit maintains content proportions
   - **Input**: content_width ∈ [1, 4096], content_height ∈ [1, 4096], viewport_width ∈ [100, 1920], viewport_height ∈ [100, 1080]
   - **Invariant**: (content_width / content_height) ≈ (scaled_width / scaled_height) when aspect_fit is applied
   - **Tolerance**: ±0.01% for aspect ratio deviation

## Input Range Strategies

### Window/Level Operations
```rust
prop::array::uniform32(
    prop::num::i16::ANY
) // For pixel data arrays

prop::num::f32::NORMAL..4096.0  // Window width (always positive)
prop::num::f32::NEG_INFINITY..2048.0  // Window level (center can be negative)
prop::num::f32::NEG_INFINITY..2048.0  // Bias adjustment
```

### Coordinate Transformations
```rust
prop::array::uniform3(
    prop::num::f32::NEG_INF..2.0 * std::f32::consts::PI
) // Euler angles (rotation around x, y, z axes)

prop::num::f32::NEG_INF..50000.0  // Coordinates (can be very large)
prop::num::f32::ANY  // Scale factors
```

### Voxel Data
```rust
prop::array::uniform3(
    prop::num::usize::MIN..=4096
) // Volume dimensions (max 4096x4096x4096)

prop::array::uniform3(
    prop::num::f32::NORMAL..100.0
) // Voxel spacing (must be positive)
```

## Test Execution Requirements

- **Minimum test cases per property**: 100 random inputs
- **Failure cases**: When property test fails, proptest shrinks to minimal counterexample
- **Coverage requirement**: ≥ 60% coverage on WindowLevel code, ≥ 65% on coordinate transformation code
- **Timeout**: 30 seconds per property test
- **Replay**: Store failing test case seeds for debugging

## Invariants Documentation

All property tests MUST verify these fundamental invariants:

1. **Numeric Precision**: Floating point operations preserve precision within defined tolerances
2. **Monotonicity**: Order-preserving functions maintain ordering
3. **Idempotency**: Applying operation twice yields same result as once (where applicable)
4. **Commutativity**: Order of operations doesn't matter (where applicable)
5. **Associativity**: Grouping of operations doesn't matter (where applicable)
6. **Identity**: Neutral element leaves operation unchanged (e.g., scale × 1.0 = original)

## Expected Outcomes

- Property tests shall pass with 100+ random inputs for each invariant
- Shrinking shall find minimal counterexamples for property violations
- Test coverage shall meet or exceed defined thresholds
- Property test failures shall provide clear diagnostics:
  - Input that failed
  - Expected invariant
  - Actual observed value
  - Minimal shrinking path

## Examples

### Window Level Monotonicity Test
```rust
proptest! {
    #[test]
    fn window_level_monotonic(width in 1.0..4096.0_f32, level1 in -2048.0..2048.0_f32, level2 in -2048.0..2048.0_f32) {
        if level1 < level2 {
            let pixel: i16 = 100;
            let output1 = apply_window_level(pixel, width, level1);
            let output2 = apply_window_level(pixel, width, level2);
            prop_assert!(output1 < output2 || output1 == output2);
        }
    }
}
```

### Coordinate Roundtrip Test
```rust
proptest! {
    #[test]
    fn coordinate_roundtrip_precision(coord in -10000.0..10000.0_f32) {
        let screen_coord = world_coord_to_screen(coord);
        let world_coord_back = screen_coord_to_world(screen_coord);
        let error = (coord - world_coord_back).abs();
        prop_assert!(error < 0.001, "Roundtrip error {} exceeds tolerance 0.001", error);
    }
}
```

## References

- Proptest documentation: https://altsysrq.github.io/proptest-book/
- Property-Based Testing: A Guide (John Hughes)
- Rust property testing patterns: https://docs.rs/proptest/latest/proptest/
