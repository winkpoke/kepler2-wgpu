## ADDED Requirements

### Requirement: v0.17.0 Public API Compatibility During Graphics Replay

The application SHALL preserve the externally reachable v0.17.0 public API while
replaying the requested dev graphics range. Private implementation interfaces MAY
change. An exported graphics signature requiring an exception SHALL be identified
and explicitly approved individually before implementation.

#### Scenario: Existing browser consumer calls the upgraded application

- **WHEN** a v0.17.0 browser consumer initializes the viewer and calls its controls
- **THEN** entry points, public class/method names, argument order, optionality, and asynchronous result shapes SHALL remain compatible
- **AND** the five-argument render-mode facade, quaternion reply shape, segmentation/OBJ/DR/primitive controls, and typed-array replies SHALL be preserved
- **AND** pre-run commands and getter replies SHALL continue through the initialized event transport

#### Scenario: Existing Rust consumer uses published domain interfaces

- **WHEN** a consumer uses release public module paths, macros, structs, enums, or traits
- **THEN** their reachable names, field/variant shapes, target availability, and caller contracts SHALL remain compatible
- **AND** default trait extensions SHALL NOT force changes to existing implementations
- **AND** no declared public item SHALL be assumed private without auditing its module and re-export reachability

#### Scenario: Modernization encounters an exported graphics incompatibility

- **WHEN** the dependency migration cannot faithfully preserve a published graphics contract
- **THEN** implementation SHALL stop at that seam pending review of its exact signature and alternatives
- **AND** an earlier exception in an unrelated integration SHALL NOT authorize the change

### Requirement: Preserve Shipped Server and Local Data Interfaces

The replay SHALL preserve the release's actual HTTP, WebSocket, CLI, native
exports, and local WASM data operations.

#### Scenario: Existing service or local viewer consumer is verified

- **WHEN** offline compatibility checks exercise published routes, DTOs, commands, and local bindings
- **THEN** methods/paths, serialized fields/variants, results, and release command syntax SHALL remain compatible
- **AND** pending backend redesign proposals SHALL NOT remove shipped interfaces during replay
- **AND** checks SHALL NOT actuate a live acquisition or navigation device
