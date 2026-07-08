# Workspace Structure Specification

## Purpose

Define the Cargo workspace and desktop crate relocation without changing desktop behavior.

## Requirements

### Requirement: Workspace Members and Desktop Relocation

The system MUST expose a Cargo workspace whose members include `helix-desktop`, `helix-core`, `helix-cli`, and `helix-ffi`. The desktop application MUST move from `src-tauri/` to `helix-desktop/` while preserving equivalent runtime behavior, packaging inputs, and app startup expectations.

#### Scenario: Workspace uses renamed desktop crate

- GIVEN the repository is checked out after the split
- WHEN Cargo resolves workspace members
- THEN `helix-desktop` is the desktop member instead of `src-tauri`

#### Scenario: User-facing behavior remains unchanged

- GIVEN an existing desktop workflow before the split
- WHEN the renamed desktop crate is built or launched
- THEN no new user-facing functional behavior is introduced by the relocation
