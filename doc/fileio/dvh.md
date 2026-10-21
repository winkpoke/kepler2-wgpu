# Imported cumulative DVHs

`data::dvh` is independent of CT, rendering and filesystem ownership.
`data::dicom::rtdvh` decodes optional source histograms from the dataset already
read by `read_rt_dose`. `DoseGrid::dvhs()` returns an optional `DvhSet` without
changing `DoseGrid::new` or replacing original u16/u32 dose samples.

`DvhSet` retains dose/structure/frame identities, validated curves and per-item
diagnostics. `validate_references` requires the exact structure-set UID, referenced
ROI numbers and any supplied frame identity. Composite INCLUDED/EXCLUDED
contributions are retained; a composite is not an individual ROI histogram.
The host owns active-object/CT/plan selection, source labels and workflow.

## Data and metric contract

- CUMULATIVE PHYSICAL curves only. GY/RELATIVE dose and CM3/PERCENT volume units
  remain explicit. No source-relative value is inferred to be prescription percent.
- Scale and accumulate dose-bin widths into f64 coordinates. Retain original,
  unsmoothed vertices. No differential/natural conversion is implicit.
- Mean/min/max are recorded exporting-system statistics. Vx/Dx queries are
  piecewise-linear bin estimates. Dx chooses the right edge of equal-volume
  plateaus. A missing tail returns None unless recorded extrema or a zero endpoint
  prove the value. Invalid/nonpositive Dx volumes return None.
- A cm³ histogram's first cumulative volume is its encoded volume basis;
  `percent_scale` converts that basis to percent. Source-percent data keep their
  original 100% basis even when their first sample is lower than 100.
  `volume_cm3` is unavailable for percent-only source data.
- Imported histogram volume is not independently reconstructed ROI volume.
  Sampling, reconstruction, grid coverage and prescription are not inferred.

Admission: 256 curves, 65,536 bins per curve, one million aggregate bins and
1–64 unique ROI contributions per curve. Dose coordinates must strictly increase;
volumes must be nonnegative and nonincreasing with positive initial volume.
Histogram vertices must fit the same finite f32 numeric range admitted by dose grids, and
percent scaling must be finite. These are model admission bounds, not peak RSS
guarantees; the existing serialized RTDOSE limit and decoder allocations remain.

Optional histogram errors do not reject valid dose pixels. Item errors preserve
other usable curves; collection errors preserve diagnostics but omit curves.
Existing constructors yield no histogram by default. This delivery does not
reconstruct structures or calculate DVHs from dose voxels.

## Verification

`cargo test --offline --locked --lib dvh` covers scaled irregular widths, original
statistics, Vx/Dx, plateau/tail semantics, percent-only basis, numerical bounds,
malformed/unsupported optional items, composite contributions, references and
collection admission. Native and WASM library types are checked through the host
delivery. The native host also verifies source replacement/removal, chart
interactions and real exported histogram ingestion without logging identifiers.

The full native library suite has nine failures reproduced on clean HEAD,
outside DVH, in camera/mesh rotation, uniform-size, window-level and CT-export
assumptions. Strict Clippy also reports existing errors outside DVH. These are
reported separately; focused DVH success does not imply the full repository gate
passes or clinical commissioning.

References: [DICOM RT DVH](https://dicom.nema.org/medical/dicom/current/output/chtml/part03/sect_C.8.8.4.html),
[AAPM TG-263](https://www.aapm.org/pubs/reports/RPT_263.pdf),
[approved change](../../openspec/changes/add-imported-dvh/proposal.md).
