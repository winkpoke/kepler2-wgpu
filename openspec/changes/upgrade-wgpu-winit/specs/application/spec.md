# Application Specification Delta

## ADDED Requirements

### Requirement: Viewer Startup Compatibility

Existing native and browser callers SHALL retain the async viewer constructor and the constructor, control-handle, event-enqueue, and run sequence. A successful constructor SHALL mean that the initial window and graphics are ready; callers SHALL NOT need a new readiness call or changed JavaScript signatures.

#### Scenario: Existing browser startup sequence
- **WHEN** a caller awaits the existing constructor, obtains its control handle, enqueues a synthetic volume, and invokes run
- **THEN** the initial graphics SHALL already be initialized when construction resolves
- **AND** the queued volume SHALL reach the running viewer without being dropped or applied twice
- **AND** the caller SHALL NOT need to change exported method names, arguments, or result shapes
- **AND** starting the browser loop SHALL return control to the browser without using a thrown exception as normal startup control flow

#### Scenario: Constructor failure
- **WHEN** initial window, adapter, device, or surface setup fails
- **THEN** the async constructor SHALL return or reject with actionable operation context
- **AND** it SHALL NOT return an apparently ready viewer and defer the same startup error until run

#### Scenario: Existing embedding and replacement
- **WHEN** a caller embeds the initial viewer or uses the existing div-targeted replacement control
- **THEN** the initial canvas SHALL retain its existing embedding target and sizing behavior
- **AND** successful replacement SHALL deliver the supplied volume through the existing control handle
- **AND** subsequent window events SHALL be routed to the active window, not the replaced window
- **AND** backend selection for a fresh canvas SHALL preserve the existing pre-binding fallback boundary

### Requirement: Viewer Control Dispatch Compatibility

The event-loop migration SHALL retain the semantics of existing implemented native input and browser control paths. Events queued before run SHALL remain deliverable in order. Read-only queries and no-op controls SHALL NOT independently schedule clean-frame redraws.

#### Scenario: Pre-run mutations
- **WHEN** volume loading and visual mutations are enqueued on a control handle before run
- **THEN** they SHALL be delivered to the viewer in enqueue order once the loop runs
- **AND** the resulting displayed state SHALL include the mutations
- **AND** initial rendering SHALL NOT require an unrelated resize or input event

#### Scenario: Existing implemented getters and setters
- **WHEN** a caller uses an implemented scalar, coordinate, rotation, or view control path
- **THEN** arguments, units, resulting state, and getter result shapes SHALL retain their existing semantics
- **AND** actual visual mutations SHALL request a complete frame
- **AND** read-only responses and no-op mutations SHALL leave a clean viewer idle

#### Scenario: Quit and close
- **WHEN** the existing quit control, native close request, or implemented escape-key path is invoked
- **THEN** the loop SHALL stop without leaving a recurring redraw or surface-retry task
- **AND** no additional rendering SHALL be scheduled after shutdown

### Requirement: Window Lifecycle Scheduling Compatibility

Window lifecycle events SHALL preserve viewer state and event-driven scheduling on supported desktop and browser targets. Repeated resume notifications SHALL NOT duplicate windows or graphics owners. Hidden, suspended, and zero-sized viewers SHALL retain pending work without continuously rendering.

#### Scenario: Initial and repeated resume
- **WHEN** an initialized viewer receives its first or a repeated resume notification
- **THEN** it SHALL retain the existing initialized window and graphics owner
- **AND** a nonzero visible surface SHALL become eligible for a complete frame
- **AND** duplicate windows or independent event loops SHALL NOT be created

#### Scenario: Suspend and restore
- **WHEN** a viewer is suspended, hidden, or resized to zero pixels and later restored
- **THEN** automatic surface acquisition SHALL stop during the unavailable interval
- **AND** pending visual changes and CPU view state SHALL be retained
- **AND** restoration SHALL schedule a complete frame with compatible resources

#### Scenario: Static idle and animation
- **WHEN** a complete frame has been presented and no mutation, animation, or recovery deadline remains
- **THEN** the viewer SHALL wait for events without repeated command submission
- **WHEN** visible animation or recoverable frame work is pending
- **THEN** subsequent rendering SHALL use bounded scheduling rather than an unconditional redraw loop

## Verification

Add deterministic tests of the production event-dispatch and lifecycle policy, including pre-run ordering, repeated resume, shutdown, and frame-result scheduling. Existing [redraw policy tests](../../../../../src/application/redraw.rs), [layout invalidation tests](../../../../../tests/redraw_state_tests.rs), and [view transition tests](../../../../../tests/view_transition_integration_tests.rs) remain regressions, but do not exercise a real window event loop.

Validate the unchanged caller sequence in [index_image.html](../../../../../static/index_image.html) and [index.html](../../../../../static/index.html), including repeat loading/replacement and implemented async getters. Native window and browser runtime checks are planned manual acceptance, not satisfied by source inspection or a WASM build; see [tasks.md](../../tasks.md).
