# Architecture Documentation Specification

## Purpose

Define the architecture documentation expectations after the workspace split.

## Requirements

### Requirement: Root Architecture Entry Point

The system MUST provide a root `ARCHITECTURE.md` that reflects the active workspace layout and stays aligned with `docs/ARCHITECTURE.md`. The documentation SHOULD explain the desktop crate role, the `helix-core` boundary, and the purpose of scaffold consumer crates without promising new product behavior.

#### Scenario: Root architecture doc exists

- GIVEN a contributor opens the repository root
- WHEN they look for architecture guidance
- THEN `ARCHITECTURE.md` is available as the top-level entry point

#### Scenario: Architecture doc matches current split

- GIVEN the workspace rename and extraction are complete
- WHEN the documentation is reviewed
- THEN the described crate boundaries match the implemented layout
