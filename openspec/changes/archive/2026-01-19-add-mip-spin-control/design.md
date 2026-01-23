## Context
The current MIP implementation renders an orthographic projection with a fixed +Z ray direction and a simplified volume intersection interval. Pan and zoom are supported via uniforms, and the application layer already routes MIP parameters (scale/pan/slab thickness/mode) through App → AppView methods driven by user events.

Adding “spin” changes the ray direction and/or sampling coordinates, which makes the current intersection logic insufficient. The change also requires an application-level control pathway similar to existing MPR/Mesh controls.

## Goals / Non-Goals
- Goals:
  - Provide manual MIP rotation control via roll/yaw/pitch inputs.
  - Preserve correctness for rays that are not axis-aligned by implementing ray-box intersection.
  - Keep uniform layout compatible across native and wasm targets.
- Non-Goals:
  - Continuous rotation animation (“auto-spin”).
  - Completing unrelated MIP features that are currently no-ops in shader (e.g., slab thickness / mode), unless required for rotation correctness.

## Decisions
- Decision: Represent rotation in the uniform buffer as a matrix suitable for WGSL.
  - Rationale: The shader needs a fast transform to rotate ray direction and/or sample coordinates.
  - Alternatives considered:
    - Store Euler angles and build a matrix in WGSL (more ALU, more branching risk).
    - Store quaternion and rotate vectors in WGSL (more math, more potential for mistakes).

- Decision: Compute ray-box intersection in the shader for the rotated ray.
  - Rationale: Without a proper intersection interval, rotated rays would either oversample outside the volume or miss valid samples.
  - Alternatives considered:
    - Keep the current fixed [0,1] interval and rely on bounds checks (simpler but incorrect edges and wasted work).
    - Precompute intersection on CPU (adds per-frame CPU work and complicates the uniform interface).

- Decision: Route rotation control through AppView as a type-safe method and expose it via a user event.
  - Rationale: Matches existing centralized view interaction patterns and avoids downcasting in the App controller.

## Risks / Trade-offs
- Numerical stability: Large angles and repeated updates could expose precision issues; prefer a stable representation and clamp/normalize where appropriate.
- Coordinate conventions: The mapping of roll/yaw/pitch axes to volume coordinates must be explicit to avoid confusion and inverted controls.
- Uniform layout: Adding matrix data changes alignment and size; incorrect layout will fail silently on some GPUs, especially on wasm targets.

## Migration Plan
1. Add rotation state and uniform fields, and update tests to validate layout.
2. Implement rotated ray generation and ray-box intersection in WGSL.
3. Add AppView and user-event routing for rotation inputs.
4. Validate on native and wasm builds using existing manual workflows.

## Open Questions
- Angle units and conventions: degrees vs radians for external API; and axis order (XYZ vs ZYX) consistency with other views.
- Rotation center: confirm rotation about the volume center (0.5, 0.5, 0.5) is desired for MIP.
- UI exposure: whether rotation is driven only via web controls initially or also via native mouse/keyboard input.
