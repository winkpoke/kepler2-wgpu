## ADDED Requirements

### Requirement: MinIP and AvgIP Rendering Modes
The MIP view shader SHALL support Minimum Intensity Projection (MinIP) and Average Intensity Projection (AvgIP) in addition to Maximum Intensity Projection (MIP).

#### Scenario: User Selects MinIP Mode
- **WHEN** the user selects MinIP mode (mode 1)
- **THEN** the shader SHALL compute the minimum intensity along the ray segment intersecting the volume
- **AND** the initial minimum intensity SHALL be initialized to a large positive value (e.g., +infinity) to support signed data (HU)
- **AND** the final pixel value SHALL be the minimum encountered value, preserving negative values
- **AND** rays not intersecting the volume SHALL return the background color
- **AND** the shader SHALL apply configurable lower and upper intensity thresholds to filter contributions based on tissue density (e.g., excluding background air or specific tissue ranges)

#### Scenario: User Selects AvgIP Mode
- **WHEN** the user selects AvgIP mode (mode 2)
- **THEN** the shader SHALL compute the average intensity along the ray segment intersecting the volume
- **AND** the sum of intensities SHALL be divided by the number of valid samples (arithmetic mean)
- **AND** the final pixel value SHALL be the computed average, preserving signed values
- **AND** rays not intersecting the volume SHALL return the background color
- **AND** the shader SHALL apply configurable lower and upper intensity thresholds to filter contributions based on tissue density (e.g., excluding background air or bone)

#### Scenario: User Selects MIP Mode
- **WHEN** the user selects MIP mode (mode 0)
- **THEN** the shader SHALL compute the maximum intensity along the ray (existing behavior)
- **AND** the initial maximum intensity SHALL be initialized to a small negative value (e.g., -infinity) to support signed data (HU)
- **AND** the final pixel value SHALL be the maximum encountered value, preserving negative values
