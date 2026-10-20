# RTDOSE import and MPR color wash

## Why
Treatment planning hosts need a quantitative dose grid independent of signed CT storage, sampled in patient coordinates.

## Approval and scope
The owner approved the proposed first delivery on 2026-10-20, including modifying Kepler and integrating the tps viewer. Import uncompressed unsigned 16/32-bit multiframe PHYSICAL dose in GY or RELATIVE units. Preserve exact samples and scaling, both frame-offset conventions, descending frames and nonuniform spacing. Render a GPU color wash in any host slice plane with independent controls and point readout.

## Non-goals
Isodose lines, DVH, reconstruction, dose calculation, summation, registration, automatic percentage normalization, single-frame grids, compressed pixels, effective/error/coded dose and clinical qualification are deferred. No new dependency is required.

## Acceptance
Serialized synthetic parser and quantitative interpolation tests; native GPU pixel comparisons; viewer routing, association and lifetime regressions; native UI, build/lint, WASM-library and release measurements with explicit evidence limits.
