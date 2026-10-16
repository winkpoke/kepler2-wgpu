# Centralize NIfTI import in Kepler

## Why
The egui adapter currently reparses NIfTI, converts scalars and interprets
geometry; browser loading instead reconstructs through MHA and loses orientation.
The owner approved the reviewed cross-repository design on 2026-10-16.

## What changes
Provide one native/bytes importer producing validated i16 CTVolume data, explicit
space provenance and diagnostics. Correct affine helpers, view centring, texture
centre mapping and geometry propagation. Make egui a thin consumer and retain
the .nii/.nii.gz picker corrections.

## Impact and acceptance
No arbitrary typed-storage redesign, global constructor validation change,
resampling, or MHA scalar policy change. Public legacy signatures remain.
Accept only when import-result tests and native rendered orientation tests pass;
record native/WASM build results and browser runtime limits separately.
