# Consumer Scaffolding Specification

## Purpose

Define the non-functional scaffold crates introduced by the workspace split.

## Requirements

### Requirement: CLI and FFI Skeleton Crates

The system MUST add `helix-cli` and `helix-ffi` as workspace members with buildable skeleton crate definitions. These crates MUST NOT introduce new end-user functionality in this change and SHALL exist only to validate future non-desktop consumers against the workspace topology.

#### Scenario: Skeleton consumers participate in workspace build

- GIVEN the workspace is built from the repository root
- WHEN Cargo includes all members
- THEN `helix-cli` and `helix-ffi` compile as scaffolding crates

#### Scenario: Skeletons do not change product scope

- GIVEN the change is reviewed for behavior impact
- WHEN CLI or FFI crates are inspected
- THEN they expose no new user-facing feature commitments in this slice
