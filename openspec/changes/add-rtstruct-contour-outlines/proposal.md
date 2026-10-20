# RTSTRUCT contour outlines

## Why
Hosts need original contours associated with the loaded CT; the existing raster label overlay cannot preserve overlapping ROIs or original polygons.

## Approval and scope
The owner approved Kepler model/parser/geometry/GPU renderer plus viewer integration on 2026-10-19, and explicitly approved modifying the sibling repository despite the viewer read-only instruction. Deliver bounded RTSTRUCT import and exact source-plane outlines. Preserve existing public constructors and volume shaders. No dependencies are added.

## Non-goals
Reconstruction between planes, filling, masks, meshes, registration, editing, export, PACS, and clinical qualification remain outside this first delivery.

## Acceptance and impact
Synthetic serialized decoder/reference rejection tests; physical-plane geometry; GPU pixels and visible native viewer acceptance; native/WASM types, formatting/lints, docs/spec checks, executable size and startup. Adds data::structures, data::dicom importer functions and rendering::contour. Native/browser hosts share the byte importer; browser runtime acceptance is reported separately.
