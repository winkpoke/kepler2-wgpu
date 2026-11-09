## Why

The viewer needs to open classic MR image stacks and identify the exact source
instances referenced by DICOM structure sets. Kepler owns the reusable importer.

## What changes

- Add incremental import of a selected classic MR series, preserving scalar samples and DICOM identity.
- Return validated voxel-centre geometry in DICOM LPS millimetres and a display window when available.
- Keep catalog selection, display policy and RT association in the consuming viewer.
- Defer compression, enhanced multiframe MR, multi-echo and temporal volume assembly, and quantitative mappings.
