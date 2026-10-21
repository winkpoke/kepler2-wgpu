# Design

Kepler `data::dvh` owns immutable cumulative vertices, source units, included /
excluded ROI references, recorded statistics and piecewise-linear Vx/Dx queries.
`data::dicom::rtdvh` parses the dataset already decoded by `read_rt_dose`; no second
file read, pixel decode or dose-grid allocation occurs. `DoseGrid::dvhs()` is an
additive accessor; existing `DoseGrid::new` callers receive no histogram by default.
Per-item errors preserve usable curves; collection errors yield diagnostics and
no curves while leaving the grid usable. Imported coverage is unknown.

Dose widths are scaled then accumulated into coordinates. No curve smoothing,
implicit differential conversion, tail extrapolation or percent-to-Gy inference
is performed. Dx uses the right edge of equal-volume plateaus. Percent-only
curves cannot provide cm³ metrics. Recorded extrema may prove a missing tail;
otherwise out-of-domain queries return None. Mean/min/max remain recorded values.

The native/WASM model is independent of egui, files, CT geometry and GPU ownership.
The host owns workers/cancellation, exact active-object association, curve colors,
visibility, selected ROI, chart range and source-aware metric display. Screen
decimation must not affect metric inputs. Cache identity includes the dose object.
Rollback removes the optional import/accessor and viewer without changing original
dose semantics. No competing viewer parser is introduced.
