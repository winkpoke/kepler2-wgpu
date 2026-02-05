## ADDED Requirements

### Requirement: Serial Connection Management
The system SHALL be able to establish and maintain a serial connection to the Remedy device.

#### Scenario: Successful connection
- **WHEN** the user initiates a connection with a valid port name
- **THEN** the system opens the serial port with the configured settings (Baud 19200, 8N1)
- **AND** sends a handshake command to verify the device presence.

#### Scenario: Connection failure
- **WHEN** the specified port cannot be opened
- **THEN** the system reports a connection error with descriptive details.

### Requirement: Acquisition Control
The system SHALL provide commands to start and stop image acquisition on the Remedy device.

#### Scenario: Start acquisition
- **WHEN** the user requests to start acquisition with specific parameters (kV, mA, exposure time)
- **THEN** the system formats the corresponding serial command
- **AND** sends it to the device
- **AND** waits for a positive acknowledgement (ACK).

### Requirement: Status Monitoring
The system SHALL listen for status updates from the Remedy device during operation.

#### Scenario: Device error reporting
- **WHEN** the device sends an error code via serial
- **THEN** the system translates the code into a human-readable error message
- **AND** logs the event.
