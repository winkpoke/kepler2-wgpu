## ADDED Requirements

### Requirement: Independent API and Behavior Gates for Rebasing

The candidate SHALL establish public API conformance and behavioral verification
against the pinned release and requested source range before destination update.

#### Scenario: Public API inventory is compared

- **WHEN** the candidate public surface is compared with the release reference
- **THEN** reachable Rust exports, fields, traits, macros, generated browser declarations, and service serialization SHALL be checked
- **AND** existing-consumer compile and dispatch/reply checks SHALL supplement textual inventories
- **AND** no unapproved public difference SHALL pass the gate

#### Scenario: Source commits are replayed

- **WHEN** the 13-commit range is rebased onto the release
- **THEN** a ledger SHALL account for all 12 ordinary commits and the recreated merge
- **AND** ordinary patch mapping and merge-specific resolutions SHALL be reviewed separately
- **AND** a dropped, split, adapted, or superseded patch SHALL be explicitly recorded

#### Scenario: Candidate build and runtime evidence is recorded

- **WHEN** native, WASM, GPU, documentation, quality, and manual checks are run
- **THEN** the report SHALL record actual commands, outcomes, compiler/backend identity, and missing environments
- **AND** historical source passes SHALL NOT be counted as candidate verification
- **AND** failures and unperformed required checks SHALL remain visible
- **AND** synthetic offline evidence SHALL NOT imply live-device or clinical acceptance

### Requirement: Isolated Replay and Protected Branches

Replay SHALL occur in a separate candidate with recoverable source/destination
refs, without modifying unrelated worktrees.

#### Scenario: Replay is abandoned or destination advances

- **WHEN** a replay fails, is aborted, or the destination moves before landing
- **THEN** source dev, release tags, and unrelated work SHALL remain intact
- **AND** the destination SHALL only advance through an accepted fast-forward candidate based on the reviewed destination
- **AND** no force update SHALL overwrite concurrent work
