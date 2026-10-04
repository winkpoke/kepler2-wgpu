# Application Specification Delta

## ADDED Requirements

### Requirement: Transactional View-Layout Transitions

A view-layout transition SHALL commit its new layout and associated orientation selections, mesh cache, and mesh-mode configuration only after all fallible preparation succeeds. Returned preparation errors SHALL preserve the previous committed viewer state and report the failed operation. Fatal GPU faults SHALL retain their existing handling.

#### Scenario: Successful transition through existing controls
- **WHEN** an existing native or browser control requests a valid view-layout transition with a loaded synthetic volume
- **THEN** the resulting arrangement SHALL follow the existing mode, view-slot, orientation, and mesh-reuse semantics
- **AND** orientations SHALL retain the existing index order: Transverse, Coronal, Sagittal, Oblique
- **AND** the new layout SHALL retain the current parent dimensions in pixels and become pending for complete-frame presentation
- **AND** the caller's method names, argument order, units, and return shape SHALL remain compatible

#### Scenario: Returned upload failure
- **WHEN** preparation of volume rendering content returns an error
- **THEN** the previous layout, view interaction state, orientation selections, mesh cache, mesh-mode configuration, and source volume SHALL remain unchanged
- **AND** clean or already-pending presentation work SHALL retain its previous status
- **AND** the error SHALL identify upload preparation and include applicable numeric dimensions or device limits without patient identifiers

#### Scenario: Returned view creation failure after partial preparation
- **WHEN** replacement view creation returns an error after other candidate views or a candidate mesh have been prepared
- **THEN** none of the candidate layout or related state SHALL become active
- **AND** the old views SHALL retain their positions, sizes, interaction state, and pending presentation status
- **AND** the error SHALL identify the requested view kind and failing operation
- **AND** abandoned candidate resources SHALL be released according to existing graphics ownership

#### Scenario: Missing volume or unusable selection
- **WHEN** a transition is requested without a loaded volume, with an out-of-range index actually used to select an orientation or view slot, or without mesh content required by its selected mode
- **THEN** the request SHALL fail before changing committed viewer state
- **AND** orientation and four-slot selections SHALL use valid indices 0 through 3 where applicable
- **AND** no request SHALL be silently stored for application after a later volume load
- **AND** failure reporting SHALL identify the missing prerequisite or unusable selection

#### Scenario: Existing mixed-layout precedence
- **WHEN** the existing controls request a mode other than single MPR, single MIP, or single mesh
- **THEN** the transition SHALL retain the existing four-slot mixed-layout interpretation
- **AND** a requested mesh and MIP targeting the same slot SHALL retain mesh precedence
- **AND** the existing explicit mesh-reuse flag SHALL retain its semantics

## MODIFIED Requirements

### Requirement: Application Testing

The application layer SHALL have comprehensive test coverage for state management, view orchestration, and user interactions to prevent regressions.

#### Scenario: Test Fixture Usage
- **WHEN** tests are written that require valid application state or configuration
- **THEN** reusable test fixtures SHALL be available via the shared `tests/common/` modules
- **AND** fixtures SHALL generate valid `AppModel`, `AppView`, and view configurations
- **AND** fixtures SHALL provide malformed data for validation tests

#### Scenario: Application State Transitions
- **WHEN** user actions modify application state (e.g., load volume, toggle mesh mode, change view layout)
- **THEN** state transitions SHALL be tested for correctness
- **AND** state SHALL remain consistent across concurrent operations
- **AND** successful view-layout transitions that recreate views SHALL use their existing initialization defaults rather than automatically restore previous interaction state
- **AND** returned transition preparation failures SHALL preserve the current layout and interaction state
- **AND** GPU rebinding of existing views SHALL preserve their CPU interaction state under its existing contract
- **AND** no source-volume data loss SHALL occur

#### Scenario: View Orchestration Testing
- **WHEN** multiple views are active (MPR, MIP, mesh)
- **THEN** view manager SHALL handle concurrent add/remove operations correctly
- **AND** active view switching SHALL follow its explicit success and failure state policy
- **AND** each view SHALL be independently testable in isolation
- **AND** memory SHALL be cleaned up when views are removed

## Verification

Implemented checks exercise the private production transition seam using [AppView headless fixtures](../../../../../src/application/appview_tests.rs), [headless GPU helpers](../../../../../tests/common/headless_gpu.rs), and [redraw regressions](../../../../../tests/redraw_state_tests.rs). Detached mode/error simulations in the [transition test file](../../../../../tests/view_transition_integration_tests.rs) were replaced by production checks; remaining ViewManager simulations do not establish this operation.

Returned-error and successful-transition execution is recorded in [verification](../../verification.md). Browser control acceptance remains pending; spec validation alone does not verify this contract.
