# Proposal: Scaffold Restructure

## Intent

The current `src-tauri/src/` layout does NOT match the target architecture defined in ARCHITECTURE.md §5.1. All Tauri commands live in `main.rs`, `AppError` is defined there, `Track` is a flat struct in `sources/`, and entire modules (`ipc/`, `playback/`, `models/`, `errors/`, etc.) are missing. This change restructures the backend to match the target folder structure so that future changes (models, IPC bridge, audio pipeline) have a proper scaffold to build on.

## Scope

### In Scope
- Restructure `src-tauri/src/` to match ARCHITECTURE.md §5.1 folder layout
- KEEP: `audio/mod.rs` (AudioBackend trait), `audio/fft.rs` (real FFT logic)
- MOVE: `AppError` → `errors/types.rs`, Tauri commands → `ipc/commands.rs`, `sources::Track` → `models/track.rs`, `sources::SourceError` → `errors/types.rs`
- ADD (empty/trait-only): `app/`, `ipc/`, `playback/`, `audio/{decoder,output,pipeline}`, `visualizer/fft_bridge`, `sources/{soundcloud,local}`, `library/`, `models/`, `persistence/`, `shared/`
- REMOVE: `plugins/` module + `wasmtime` + `wgpu` deps from Cargo.toml (not v0.1 per PRD)
- BOOTSTRAP: `cargo test` infrastructure (at least one `#[cfg(test)]` module)
- Update `main.rs` to use new module paths only

### Out of Scope
- Frontend restructure (change #2: `frontend-restructure`)
- Rich data models — Track/Artist/Album with Source enum, Option fields, HashMap (change #3: `models-and-errors`)
- IPC events layer + `app/setup.rs` builder (change #4: `ipc-bridge`)
- Actual audio playback implementation
- Source resolver implementations (YouTube, SoundCloud, Local)

## Capabilities

### New Capabilities
- `backend-scaffold`: Module layout and public interfaces for all backend domains per ARCHITECTURE.md §5.1

### Modified Capabilities
- None (no existing specs to modify — this is the first SDD change)

## Approach

Incremental module-by-module restructure, keeping `cargo build` passing after each step:

1. **Add empty modules** — create all missing directories with `mod.rs` stubs
2. **Move types** — relocate `AppError`, `Track`, `SourceError` to their target modules, add `pub use` re-exports in origin modules for backward compat
3. **Move Tauri commands** — extract from `main.rs` to `ipc/commands.rs`, update `invoke_handler` registration
4. **Remove plugins + premature deps** — delete `plugins/`, remove `wasmtime`/`wgpu` from Cargo.toml, remove `visualizer/renderer.rs` (wgpu-dependent)
5. **Slim `main.rs`** — entry point only: `mod` declarations + `tauri::Builder` launch
6. **Bootstrap tests** — add `#[cfg(test)]` module in `errors/types.rs`, verify `cargo test` runs
7. **Remove re-exports** — once all imports point to new locations, remove backward-compat `pub use` aliases

## Affected Areas

| Area | Impact | Description |
|------|--------|-------------|
| `src-tauri/src/main.rs` | Modified | Slim to entry point only; commands move to `ipc/` |
| `src-tauri/src/audio/mod.rs` | Modified | Add submodules: decoder, output, pipeline |
| `src-tauri/src/audio/fft.rs` | Kept | Stays in place |
| `src-tauri/src/audio/playback.rs` | Modified | Split into decoder.rs + output.rs stubs |
| `src-tauri/src/errors/` | New | `mod.rs` + `types.rs` (AppError + SourceError relocated) |
| `src-tauri/src/models/` | New | `mod.rs` + `track.rs` (flat Track relocated), `source.rs` stub |
| `src-tauri/src/ipc/` | New | `mod.rs` + `commands.rs` (Tauri commands), `events.rs` stub |
| `src-tauri/src/app/` | New | `mod.rs` + `setup.rs` stubs |
| `src-tauri/src/playback/` | New | `mod.rs` + service/state/events/models stubs |
| `src-tauri/src/visualizer/` | Modified | Remove renderer.rs, add fft_bridge.rs stub |
| `src-tauri/src/sources/` | Modified | Add soundcloud/ + local/ submodules |
| `src-tauri/src/library/` | New | `mod.rs` + service/state/models stubs |
| `src-tauri/src/persistence/` | New | `mod.rs` + `db.rs` stub |
| `src-tauri/src/shared/` | New | `mod.rs` + `utils.rs` stub |
| `src-tauri/src/plugins/` | Removed | Not v0.1 per PRD |
| `src-tauri/Cargo.toml` | Modified | Remove wasmtime + wgpu deps |

## Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| Compilation breaks during module moves | Med | Restructure one module at a time, `cargo check` after each |
| Import path cascades | Med | Use `pub use` re-exports temporarily, remove after all imports updated |
| `wasmtime`/`wgpu` removal breaks `visualizer/renderer.rs` | High | Delete renderer.rs alongside dep removal — it's an empty stub |
| `edition = "2024"` compatibility | Low | Verify `cargo check` passes before starting |
| Frontend `App.svelte` breaks on IPC shape change | Med | Acceptable — prototype code, will be restructured in change #2 |

## Rollback Plan

`git revert` on the feature branch. Each step is a separate commit, so individual steps can be reverted independently. The `main` branch is not affected until merge.

## Dependencies

- None (this is change #1 of 4)

## Success Criteria

- [ ] `cargo build` compiles with zero errors after restructure
- [ ] `cargo test` runs (at least one test passes)
- [ ] Folder structure matches ARCHITECTURE.md §5.1
- [ ] `plugins/` module and `wasmtime`/`wgpu` deps removed from Cargo.toml
- [ ] `main.rs` contains only mod declarations + tauri::Builder launch
- [ ] No `pub use` backward-compat re-exports remain (clean import paths)