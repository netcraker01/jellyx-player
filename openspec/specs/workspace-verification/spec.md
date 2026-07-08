# Workspace Verification Specification

## Purpose

Define mandatory verification outcomes for the workspace split.

## Requirements

### Requirement: Build and Runtime Verification

The system MUST verify that `cargo check -p helix-desktop` succeeds after the desktop move and that `cargo build --workspace` succeeds after extraction and scaffolding. The change MUST attempt `cargo run -p helix-desktop`; if runtime confirmation is blocked by the GUI environment, the verification report SHALL state that limitation explicitly.

#### Scenario: Required build commands succeed

- GIVEN the workspace split is implemented
- WHEN the required Cargo verification commands are executed
- THEN the desktop package check and full workspace build succeed

#### Scenario: GUI runtime cannot be silently skipped

- GIVEN desktop runtime confirmation depends on a GUI-capable environment
- WHEN `cargo run -p helix-desktop` cannot be fully confirmed
- THEN the change record explicitly reports the blocking environment constraint
