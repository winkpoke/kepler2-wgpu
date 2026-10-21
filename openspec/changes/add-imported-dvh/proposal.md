# Imported DVH review

Approved by the owner in the viewer chat on 2026-10-21, including explicit
authorization to update sibling Kepler for DVH parsing and metrics.

Preserve optional cumulative PHYSICAL histograms embedded in RTDOSE, expose
their exact ROI/structure references and recorded statistics, and provide f64
bin-derived Vx/Dx queries. The host viewer supplies MPR/DVH layouts and source
labels. Optional malformed or unsupported histograms must not discard valid dose
pixels. Admission bounds and typed diagnostics apply to every curve.

No contour-volume reconstruction, local DVH recalculation, biological modelling,
prescription normalization, clinical-goal evaluation or dose summation is added.
Existing constructors and image/GPU interfaces remain compatible. No dependencies
or texture formats change. Acceptance includes synthetic parsing, interpolation,
plateau/tail/unit semantics, reference mismatches, viewer state/UI checks, native
and WASM types and measured viewer release size/startup.
