# Application Specification Delta

## ADDED Requirements

### Requirement: Event-Driven Redraw Scheduling

The application SHALL schedule redraws for pending visual changes, explicit surface redraw requests, and active visible animation rather than unconditionally requesting another frame after each redraw. Static idle application state SHALL NOT drive recurring rendering or command submission.

#### Scenario: Static idle viewer
- **WHEN** the last required frame has been successfully presented and no visible animation or pending retry exists
- **THEN** the application SHALL wait for events without requesting another frame
- **AND** it SHALL NOT repeatedly update view uniforms or submit rendering commands

#### Scenario: Visual controls and linked changes
- **WHEN** volume, slice, window/level, orientation, transform, projection, mesh, or layout state changes visibly
- **THEN** the affected state SHALL be invalidated
- **AND** the application SHALL schedule a redraw
- **AND** linked slice changes SHALL be included in the pending frame
- **AND** multiple pending mutations MAY be coalesced into one frame

#### Scenario: No-op controls or read-only queries
- **WHEN** a control leaves the effective displayed state unchanged or a read-only query occurs
- **THEN** that operation SHALL NOT independently invalidate a clean frame or schedule a redraw

#### Scenario: Layout cleared
- **WHEN** all views are removed from the layout
- **THEN** the application SHALL schedule a complete frame displaying the empty-layout background
- **AND** stale pixels from removed views SHALL NOT remain displayed

#### Scenario: Animation enabled and disabled
- **WHEN** a visible mesh animation is enabled
- **THEN** subsequent frames SHALL be scheduled at a bounded cadence while animation requires them
- **AND** enabling animation after idle SHALL reset elapsed-time bookkeeping as necessary
- **WHEN** the animation stops and the final required frame succeeds
- **THEN** automatic redraws SHALL cease unless other work is pending

#### Scenario: Initial display and explicit surface redraw
- **WHEN** the application starts with a configured nonzero surface or receives an OS redraw request
- **THEN** it SHALL present a complete frame without requiring a prior visual control mutation

#### Scenario: Resize and graphics recovery
- **WHEN** a nonzero surface resize, restoration, surface recreation, or graphics replacement occurs
- **THEN** the application SHALL invalidate the frame and schedule rendering with compatible resources

#### Scenario: Minimized window
- **WHEN** the window has a zero width or height
- **THEN** automatic redraws and surface acquisition SHALL be suppressed
- **AND** pending visual changes SHALL be retained for restoration

#### Scenario: Recoverable surface error
- **WHEN** surface acquisition reports lost, outdated, or timeout
- **THEN** required visual work SHALL remain pending
- **AND** reconfiguration or a bounded retry SHALL be scheduled as appropriate
- **AND** out-of-memory SHALL retain the existing exit behavior

#### Scenario: Deferred renderer recovery
- **WHEN** an existing renderer fallback requires a timed recovery attempt
- **THEN** the application SHALL schedule the necessary recovery deadline without continuously rendering otherwise idle views
