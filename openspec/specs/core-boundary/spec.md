# Core Boundary Specification

## Purpose

Define which Rust logic may move into `helix-core`.

## Requirements

### Requirement: Tauri-Free Core Extraction

The system MUST place only Tauri-free business, data, and music-domain logic in `helix-core`. The system MUST keep Tauri-bound concerns, including `AppHandle`, command handlers, event emission, IPC channels, and FFT bridge integration, in `helix-desktop` unless an explicit adapter preserves the boundary.

#### Scenario: Pure domain modules move to core

- GIVEN a Rust module has no Tauri runtime dependency
- WHEN the workspace is refactored
- THEN that module MAY be owned by `helix-core`

#### Scenario: Tauri integration stays desktop-side

- GIVEN a module depends on Tauri runtime or window/app APIs
- WHEN the extraction is evaluated
- THEN that module MUST remain in `helix-desktop` or behind a safe adapter
