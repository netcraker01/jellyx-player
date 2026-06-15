# Backend Scaffold Specification

## Purpose

Module layout and public interfaces for all backend domains per ARCHITECTURE.md §5.1. This change restructures `src-tauri/src/` to match the target architecture, relocates misplaced types, removes premature dependencies, and bootstraps test infrastructure.

## Requirements

### SR-001: Module Layout Conformance

The backend directory structure under `src-tauri/src/` MUST match ARCHITECTURE.md §5.1. Every domain module listed there SHALL exist as a directory with a `mod.rs`. Submodule files listed in §5.1 MUST exist (may be stubs).

#### Scenario: All domain modules present

- GIVEN the target layout defined in ARCHITECTURE.md §5.1
- WHEN `cargo build` is invoked
- THEN directories `app/`, `ipc/`, `playback/`, `audio/`, `visualizer/`, `sources/`, `library/`, `models/`, `persistence/`, `errors/`, `shared/` each contain a `mod.rs`
- AND each `mod.rs` declares its documented submodules

#### Scenario: Audio submodules added

- GIVEN `audio/mod.rs` currently declares only `playback` and `fft`
- WHEN restructured
- THEN `audio/` also declares `decoder`, `output`, and `pipeline` submodules

#### Scenario: Sources submodules added

- GIVEN `sources/mod.rs` currently declares only `youtube`
- WHEN restructured
- THEN `sources/` also declares `soundcloud` and `local` submodules

### SR-002: Type Relocation — AppError and SourceError

`AppError` (currently in `main.rs`) and `SourceError` (currently in `sources/mod.rs`) MUST be relocated to `errors/types.rs`. The `From` impls for `AppError` SHALL move with them.

#### Scenario: AppError accessible from errors module

- GIVEN `AppError` is defined in `main.rs`
- WHEN `AppError` is moved to `errors::types`
- THEN `errors::types::AppError` is the canonical path
- AND `main.rs` no longer defines `AppError`

#### Scenario: SourceError accessible from errors module

- GIVEN `SourceError` is defined in `sources::mod`
- WHEN `SourceError` is moved to `errors::types`
- THEN `errors::types::SourceError` is the canonical path
- AND `sources::mod` no longer defines `SourceError`

### SR-003: Type Relocation — Track

`Track` (currently in `sources/mod.rs`) MUST be relocated to `models/track.rs`. `sources/mod.rs` SHALL NOT define `Track` after relocation.

#### Scenario: Track accessible from models module

- GIVEN `Track` is defined in `sources::mod`
- WHEN `Track` is moved to `models::track`
- THEN `models::track::Track` is the canonical path
- AND `sources::mod` no longer defines `Track`

#### Scenario: Existing Track usages compile

- GIVEN `ipc::commands` and `sources::youtube` reference `Track`
- WHEN `Track` is relocated to `models::track`
- THEN all references use `models::track::Track` and compile without error

### SR-004: Command Extraction to IPC

All `#[tauri::command]` functions currently in `main.rs` MUST be moved to `ipc/commands.rs`. `AppState` struct SHALL move to `ipc/commands.rs` alongside them. `main.rs` invoke_handler registration MUST reference `ipc::commands::*`.

#### Scenario: Commands live in IPC module

- GIVEN `search`, `play`, `pause`, `resume`, `seek`, `volume`, `version` are defined in `main.rs`
- WHEN extracted to `ipc::commands`
- THEN all seven commands are defined in `ipc/commands.rs`
- AND `main.rs` registers them via `ipc::commands::*`

#### Scenario: AppState co-located with commands

- GIVEN `AppState` struct is in `main.rs`
- WHEN commands are extracted
- THEN `AppState` moves to `ipc/commands.rs` alongside the commands that use it

### SR-005: Dependency Cleanup — Remove Premature Deps

`wasmtime` and `wgpu` MUST be removed from `Cargo.toml` dependencies. The `plugins/` module directory and `visualizer/renderer.rs` MUST be deleted. No code SHALL reference `wasmtime` or `wgpu` after removal.

#### Scenario: Cargo.toml cleaned

- GIVEN `Cargo.toml` contains `wasmtime = "19"` and `wgpu = "22"`
- WHEN dependencies are cleaned
- THEN neither `wasmtime` nor `wgpu` appear in `[dependencies]`
- AND `cargo build` succeeds

#### Scenario: Plugins module removed

- GIVEN `src-tauri/src/plugins/` directory exists
- WHEN plugins are removed
- THEN `plugins/` directory does not exist
- AND `main.rs` does not declare `mod plugins`

#### Scenario: Visualizer renderer removed

- GIVEN `visualizer/renderer.rs` depends on `wgpu`
- WHEN `wgpu` is removed
- THEN `visualizer/renderer.rs` is deleted
- AND `visualizer/mod.rs` no longer declares `mod renderer`

### SR-006: Test Infrastructure Bootstrap

At least one `#[cfg(test)]` module MUST exist. `cargo test` SHALL run and pass at least one test.

#### Scenario: Test module exists and passes

- GIVEN no `#[cfg(test)]` module exists in the codebase
- WHEN test infrastructure is bootstrapped
- THEN `errors/types.rs` (or another module) contains a `#[cfg(test)] mod tests` block
- AND `cargo test` exits with code 0
- AND at least one test function passes

### SR-007: main.rs Slimming

`main.rs` MUST contain ONLY `mod` declarations and the `tauri::Builder` launch call. No type definitions, trait impls, or command functions SHALL remain in `main.rs`.

#### Scenario: main.rs is entry-only

- GIVEN `main.rs` defines `AppError`, `AppState`, and seven command functions
- WHEN slimmed
- THEN `main.rs` contains only `mod` declarations and `fn main()` with `tauri::Builder`
- AND no `struct`, `enum`, `impl`, or `#[tauri::command]` blocks exist in `main.rs`

### SR-008: Re-export Cleanup

No temporary `pub use` backward-compatibility re-exports SHALL remain after all import paths are updated to their canonical locations.

#### Scenario: No backward-compat aliases

- GIVEN types were moved with temporary `pub use` bridges
- WHEN all consumer modules import from canonical paths
- THEN no `pub use` re-exports for relocated types remain in origin modules