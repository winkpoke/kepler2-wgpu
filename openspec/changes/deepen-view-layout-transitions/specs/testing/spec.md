# Testing Specification Delta

## MODIFIED Requirements

### Requirement: View Manager State Consistency

View manager SHALL maintain consistent state across concurrent operations and ensure proper resource cleanup. View-layout transition checks SHALL exercise the production operation and distinguish successful view recreation from failed preparation and GPU rebinding.

#### Scenario: Concurrent View Add/Remove Operations
- **WHEN** multiple threads add and remove views simultaneously
- **THEN** the view count SHALL always be accurate
- **AND** race conditions SHALL NOT cause view count errors
- **AND** removed views SHALL be fully cleaned up (no memory leaks)
- **AND** active view pointer SHALL remain valid

#### Scenario: View State Restoration
- **WHEN** a view-layout transition returns a preparation error while switching between grid and single-view layouts
- **THEN** the previous displayed layout and view count SHALL remain unchanged
- **AND** slice positions, pan, and scale SHALL be preserved
- **AND** orientation and window/level settings SHALL be maintained
- **AND** rotation, view positions and sizes in pixels, and clean or pending presentation status SHALL remain unchanged
- **AND** no source-volume data loss SHALL occur

#### Scenario: Successful view recreation resets interaction state
- **WHEN** a successful view-layout transition recreates views after existing interaction settings have been changed
- **THEN** production-seam checks SHALL verify the new views use their existing initialization defaults
- **AND** previously captured interaction state SHALL NOT be restored automatically
- **AND** view-slot orientation selection and source-volume data SHALL follow the requested existing transition semantics

#### Scenario: Existing views retain state on GPU rebinding
- **WHEN** GPU resources are rebound for existing views without recreating their layout
- **THEN** the existing CPU-state preservation and compatible-resource-sharing regressions SHALL remain active
- **AND** the successful layout-recreation reset policy SHALL NOT apply to that operation

#### Scenario: Returned failure adapters exercise the live sequence
- **WHEN** transition tests inject upload or view creation errors
- **THEN** they SHALL exercise the same production preparation and commit sequence as native and browser controls
- **AND** they SHALL cover failure after partial candidate construction and pending related-state updates
- **AND** tests that only count workflow labels or change a test-local transition enum SHALL NOT be treated as verification of this operation

## Verification

Use the [existing synthetic headless fixtures](../../../../../src/application/appview_tests.rs) and [GPU helper](../../../../../tests/common/headless_gpu.rs) for real construction and offscreen pixel checks. Retain [rendering correctness](../../../../../tests/rendering_correctness_tests.rs), [resource lifetime](../../../../../tests/render_resource_lifetime_tests.rs), and [redraw](../../../../../tests/redraw_state_tests.rs) regressions.

The modified restoration scenario resolves the explicit baseline conflict with the reset behavior accepted by the user on 2026-10-04. Concurrent operations and driver-memory claims retain their existing requirements and verification gaps; this change does not claim to establish them. Production transition checks pass as recorded in [verification](../../verification.md); browser runtime acceptance remains pending.
