## ADDED Requirements

### Requirement: Imported cumulative DVH data
Kepler SHALL retain optional CUMULATIVE PHYSICAL DVHs from the already-decoded
RTDOSE dataset, including source dose/structure identities, source units,
included/excluded ROI contributions, original bin vertices and recorded statistics.
It SHALL scale and accumulate dose-bin widths and SHALL admit no more than 256
curves, 65,536 bins per curve and 1,000,000 aggregate bins per dose object.

#### Scenario: Valid optional histograms
- **WHEN** a physical RTDOSE includes valid cumulative histograms
- **THEN** `DoseGrid::dvhs()` SHALL expose them without replacing original dose samples
- **AND** existing `DoseGrid::new` callers SHALL remain compatible.

#### Scenario: Malformed or unsupported optional data
- **WHEN** a histogram is malformed, differential, natural or non-PHYSICAL
- **THEN** that histogram SHALL be omitted with an indexed diagnostic
- **AND** usable dose pixels and other valid admitted histograms SHALL remain available.

### Requirement: Source-aware DVH metrics
Kepler SHALL evaluate Vx and Dx in f64 from piecewise-linear cumulative vertices,
using the right edge of equal-volume plateaus for Dx. It SHALL return no result
for unbracketed tails unless recorded extrema or a zero endpoint prove the value.
It SHALL keep recorded statistics distinct from estimates and SHALL NOT infer
absolute volumes from percent-only data, coverage, prescription or dose summation.

#### Scenario: Metric input and output units
- **WHEN** a caller requests D95% or V20 in the source dose units
- **THEN** the metric SHALL use the histogram's declared volume basis and original bins
- **AND** invalid, unavailable or out-of-domain queries SHALL return None.

#### Scenario: Structure association
- **WHEN** a host validates a histogram against a structure set
- **THEN** the exact structure-set identity, every referenced ROI number and any
  supplied frame identity SHALL match before association succeeds.
