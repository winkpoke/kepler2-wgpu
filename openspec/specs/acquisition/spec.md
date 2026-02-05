# Acquisition Capability Specification

### Requirement: Serial Connection Management
The system SHALL establish and maintain a serial connection to the Remedy device.

#### Scenario: Web connection
- **WHEN** the user connects via browser
- **THEN** the system uses the Web Serial API (baud 19200, 8N1)
- **AND** sends a handshake command to verify device presence.

#### Scenario: Native connection
- **WHEN** the user connects via native application
- **THEN** the system uses a platform serial interface
- **AND** applies the same protocol framing and handshake.

#### Scenario: Connection failure
- **WHEN** the specified port cannot be opened
- **THEN** the system reports a descriptive connection error.

### Requirement: Acquisition Control
The system SHALL provide commands to start and stop exposure on the Remedy device.

#### Scenario: Start exposure
- **WHEN** the user requests to start exposure with parameters (KV, MA, MS)
- **THEN** the system formats the corresponding command packet
- **AND** sends it to the device
- **AND** expects positive acknowledgement (ACK).

#### Scenario: Cancel/Prep
- **WHEN** the user requests cancel or prep
- **THEN** the system sends PR0/PR1 commands with correct checksum and ETX.

### Requirement: Status Monitoring
The system SHALL process and present device status updates.

#### Scenario: Status updates
- **WHEN** the device reports KV/MA/MS/MX/Status/Heat values
- **THEN** the system parses fields and updates UI state.

#### Scenario: Error reporting
- **WHEN** the device sends an error code
- **THEN** the system translates to readable message
- **AND** logs the event.
