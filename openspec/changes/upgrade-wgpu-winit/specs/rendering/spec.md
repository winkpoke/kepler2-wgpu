# Rendering Specification Delta

## ADDED Requirements

### Requirement: Application-Owned Rendering Failures

Public Rust rendering APIs SHALL report failures through application-owned error types, not a backend surface error type. Errors SHALL identify the failing operation without patient-identifying data. View/pass failures SHALL remain distinguishable from presentation-surface failures.

#### Scenario: Downstream custom view
- **WHEN** a downstream caller implements the public rendering trait
- **THEN** its render method SHALL return the application's result/error contract
- **AND** the migration guidance SHALL identify the changed Rust signature and required imports
- **AND** successful drawing SHALL retain a unit result
- **AND** the public pass context SHALL separate its mutable-borrow and encoder lifetimes, with both lifetime arguments documented for explicitly annotated downstream callers

#### Scenario: View or pass failure
- **WHEN** drawing a view or executing a pass fails
- **THEN** the caller SHALL receive an actionable application-owned rendering error
- **AND** the failure SHALL NOT be relabeled as a lost presentation surface
- **AND** pending presentation state SHALL NOT be consumed

#### Scenario: Public frame API compatibility
- **WHEN** a Rust caller invokes the public frame-rendering method
- **THEN** successful rendering SHALL retain a unit result
- **AND** recoverable acquisition failures SHALL remain inspectable through application-owned surface failure classifications
- **AND** backend-owned frame textures or acquisition enums SHALL NOT become the public error payload

### Requirement: Surface Frame Outcome Handling

The renderer SHALL distinguish presented frames, skipped acquisitions, recoverable surface conditions, and unrecoverable graphics failures. Pending visual changes SHALL be cleared only after complete-frame submission and presentation handoff. Recoverable conditions SHALL NOT cause an immediate retry loop.

#### Scenario: Successful acquisition
- **WHEN** a configured nonzero surface supplies a usable frame
- **THEN** all visible views SHALL be drawn into a complete frame
- **AND** commands SHALL be submitted before the frame is handed off for presentation
- **AND** that frame SHALL be presented exactly once

#### Scenario: Suboptimal acquisition
- **WHEN** acquisition supplies a usable but suboptimal frame
- **THEN** that frame SHALL remain drawable and SHALL be presented at most once
- **AND** compatible surface reconfiguration SHALL be scheduled without losing subsequent visual changes
- **AND** unchanged device and attachment format SHALL NOT independently require pipeline reconstruction

#### Scenario: Acquisition timeout
- **WHEN** acquisition times out
- **THEN** no frame SHALL be presented for that attempt
- **AND** pending visual changes SHALL remain pending
- **AND** the retry SHALL use the existing bounded scheduling cadence rather than immediate repeated acquisition

#### Scenario: Occluded or zero-sized surface
- **WHEN** a surface is reported occluded or has zero width or height in pixels
- **THEN** no presentation SHALL be reported as successful for the skipped attempt
- **AND** automatic acquisition SHALL be suppressed while the window is known hidden, suspended, or zero-sized
- **AND** restoration SHALL permit the retained visual changes to be rendered

#### Scenario: Outdated surface
- **WHEN** acquisition reports an outdated surface on a healthy device
- **THEN** compatible nonzero surface reconfiguration SHALL precede a scheduled retry
- **AND** same-device, same-format resources SHALL remain reusable
- **AND** pending visual changes SHALL remain pending

#### Scenario: Lost surface
- **WHEN** the presentation surface is lost but the selected device remains usable
- **THEN** recovery SHALL recreate and configure the surface using the selected backend before a scheduled retry
- **AND** CPU view state SHALL remain intact
- **AND** browser recovery SHALL NOT switch the already-bound canvas to another graphics API
- **AND** failed recovery SHALL report its backend and operation rather than retry immediately without bound

#### Scenario: Validation or fatal device failure
- **WHEN** acquisition reports validation failure or a graphics operation reports out-of-memory or device loss
- **THEN** the application SHALL report actionable failure context and stop automatic rendering/recovery for that failure
- **AND** out-of-memory SHALL retain the existing exit behavior
- **AND** the application SHALL NOT mistake the failure for a routine outdated surface
- **AND** no successful presentation SHALL be recorded for the failed attempt

### Requirement: Surface Presentation Color Compatibility

Surface presentation SHALL preserve the existing standard-dynamic-range grayscale policy. When supported, initialization SHALL prefer a non-sRGB attachment format; otherwise it SHALL retain the supported-format fallback. A dependency migration SHALL NOT introduce HDR, wide-gamut output, or an extra display transform.

#### Scenario: Surface configuration selection
- **WHEN** a surface reports supported presentation formats
- **THEN** selection SHALL prefer the first supported non-sRGB format or, if none exists, the first supported format
- **AND** configuration SHALL use historical standard-dynamic-range color-space behavior
- **AND** an empty supported-format set SHALL return an initialization error

#### Scenario: Signed grayscale regression
- **WHEN** the existing synthetic signed-value rendering fixtures run with either supported volume encoding
- **THEN** MPR and projection pixels SHALL remain within their existing asserted tolerances
- **AND** packed signed storage, half-float rounding, window/level, matrix layout, and projection-mode semantics SHALL remain unchanged
- **AND** tolerances SHALL NOT be widened merely to accept changed dependency output

## Verification

Extend the production acquisition/recovery path with deterministic fault-driven regressions; these are planned, not implemented. Preserve [pixel and format/device regressions](../../../../../tests/rendering_correctness_tests.rs), [signed encoding regressions](../../../../../tests/volume_encoding_tests.rs), and [resource ownership regressions](../../../../../tests/render_resource_lifetime_tests.rs). Existing signed grayscale tests use a one-byte framebuffer tolerance; device/format rebinding uses two bytes. Use each test's actual bound rather than a new global tolerance.

Native offscreen tests do not exercise real presentation surfaces or browser color management. Native surface behavior and independent WebGPU/WebGL2 checks remain manual acceptance in [tasks.md](../../tasks.md), following the [browser procedure](../../../../../doc/testing/browser_gpu.md). Presentation handoff is not proof of physical display completion or clinical readiness.
