# Change: Add Remedy Serial Communication for Image Acquisition

## Why
The system needs to interface with the Remedy hardware for image acquisition. This requires establishing a serial communication channel to send commands and receive status/data from the device.

## What Changes
- Create a new `acquisition` capability to handle hardware interactions.
- Implement a serial communication driver specifically for the Remedy protocol.
- Add `serialport` crate dependency (or similar) for cross-platform serial IO.
- Define a `RemedyDevice` interface for higher-level application logic to control acquisition.

## Impact
- **New Capability**: `acquisition`
- **Dependencies**: Adds `serialport` (or equivalent).
- **Architecture**: Introduces a hardware abstraction layer for acquisition devices.
