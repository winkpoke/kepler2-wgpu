# Core Specification

## Purpose

Defines requirements for Kepler's core mathematics, coordinate systems, geometry, and window/level behavior.

## Requirements

### Requirement: Core Testing

The core module SHALL have comprehensive test coverage for mathematical operations, coordinate systems, and geometry transformations to ensure numerical correctness.

#### Scenario: Math Operation Testing
- **WHEN** core math functions are used (matrix operations, vector math, transformations)
- **THEN** operations SHALL be tested with property-based tests using `proptest`
- **AND** edge cases SHALL be covered (zero values, infinity, NaN)
- **AND** numerical precision SHALL be verified across all input ranges
- **AND** performance SHALL meet expected thresholds

#### Scenario: Coordinate System Testing
- **WHEN** coordinate transformations are performed (world ↔ screen ↔ voxel)
- **THEN** correctness SHALL be verified through roundtrip tests
- **AND** coordinate bounds SHALL be tested (negative values, overflow, NaN)
- **AND** orientation matrices SHALL be tested for orthogonality
- **AND** precision SHALL be preserved (error accumulation < 0.001 world units)

#### Scenario: Window/Level Testing
- **WHEN** window/level transformations are applied to CT values
- **THEN** mathematical properties SHALL be verified (midpoint property, clamping behavior)
- **AND** extreme values SHALL be tested to ensure proper clamping
- **AND** precision SHALL be preserved across all valid ranges
- **AND** transformations SHALL preserve contrast ratios

#### Scenario: Geometry Testing
- **WHEN** geometry bases are constructed (axial, coronal, sagittal, oblique)
- **THEN** matrices SHALL be tested for correct orientation and scale
- **AND** determinants SHALL be verified (orthogonal matrices have det = ±1)
- **AND** basis vectors SHALL be normalized to unit length
- **AND** transformations SHALL produce correct screen/world mappings
