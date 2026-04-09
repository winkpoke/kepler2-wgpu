## 1. Implementation
- [ ] 1.1 Update dual MPR composition logic to render axial and sagittal slices in the same full canvas (no viewport split).
- [ ] 1.2 Update fragment shader composition order so axial is rendered as base and sagittal is blended on top.
- [ ] 1.3 Add or reuse opacity uniform(s) for sagittal overlay with a safe default and optional user-defined control.
- [ ] 1.4 Ensure transfer function and window/level mapping are applied consistently to both slices before blending.
- [ ] 1.5 Keep pipeline and uniform layout changes minimal while preserving 16-byte alignment compatibility.
- [ ] 1.6 Validate rendering quality (artifact-free overlay) and baseline performance for large/high-resolution volumes.
- [ ] 1.7 Add/extend tests for blend behavior, uniform updates, and dual-mode stability in native and wasm builds.
