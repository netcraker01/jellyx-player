# Tasks: Scaffold Restructure

## Review Workload Forecast

| Field | Value |
|-------|-------|
| Estimated changed lines | 300–400 |
| 400-line budget risk | Medium |
| Chained PRs recommended | No |
| Suggested split | Single PR |
| Delivery strategy | auto-chain |
| Chain strategy | size-exception |

Decision needed before apply: No
Chained PRs recommended: No
Chain strategy: size-exception
400-line budget risk: Medium

### Suggested Work Units

| Unit | Goal | Likely PR | Notes |
|------|------|-----------|-------|
| 1 | Full scaffold restructure | PR 1 | Single PR; structural moves + stubs, not feature work |

## Phase 1: Foundation — Empty Module Stubs

- [x] 1.1 Create `src-tauri/src/errors/mod.rs` with `pub mod types;` and `src-tauri/src/errors/types.rs` empty stub
- [x] 1.2 Create `src-tauri/src/models/mod.rs` declaring `pub mod track; pub mod source;` + empty `models/track.rs`, `models/source.rs`
- [x] 1.3 Create `src-tauri/src/ipc/mod.rs` declaring `pub mod commands; pub mod events;` + empty stubs
- [x] 1.4 Create `src-tauri/src/app/mod.rs` declaring `pub mod setup;` + empty `app/setup.rs`
- [x] 1.5 Create `src-tauri/src/playback/` with `mod.rs` (service, state, events, models) + 4 empty stubs
- [x] 1.6 Add `pub mod decoder; pub mod output; pub mod pipeline;` to `audio/mod.rs` + 3 empty stubs
- [x] 1.7 Create `src-tauri/src/library/` with `mod.rs` (service, state, models) + 3 empty stubs
- [x] 1.8 Create `src-tauri/src/persistence/mod.rs` + `persistence/db.rs` stub
- [x] 1.9 Create `src-tauri/src/shared/mod.rs` + `shared/utils.rs` stub
- [x] 1.10 Create `src-tauri/src/sources/soundcloud/mod.rs` + `sources/local/mod.rs` stubs
- [x] 1.11 Create `src-tauri/src/visualizer/fft_bridge.rs` stub
- [x] 1.12 Add `mod` declarations for all new top-level modules in `main.rs` → `cargo check`

## Phase 2: Type Relocation

- [x] 2.1 Move `AppError` struct + `From<SourceError>` + `From<AudioError>` impls from `main.rs` to `errors/types.rs`
- [x] 2.2 Move `SourceError` enum from `sources/mod.rs` to `errors/types.rs`; add `pub use` bridge in `sources/mod.rs`
- [x] 2.3 Move `Track` struct from `sources/mod.rs` to `models/track.rs`; add `pub use` bridge in `sources/mod.rs`
- [x] 2.4 Update `sources/youtube.rs` to import `Track` from `crate::models::track` and `SourceError` from `crate::errors::types`
- [x] 2.5 Update `main.rs` to import `AppError` from `crate::errors::types` → `cargo check`

## Phase 3: Command + CpalBackend Extraction

- [x] 3.1 Move `AppState` struct + all 7 `#[tauri::command]` fns from `main.rs` to `ipc/commands.rs`; update imports
- [x] 3.2 Update `main.rs` `invoke_handler` to register `ipc::commands::*`; add `use` for `CpalBackend`
- [x] 3.3 Move `CpalBackend` from `audio/playback.rs` to `audio/output.rs`; keep `AudioBackend` trait + `PlaybackState` + `AudioError` in `audio/mod.rs`
- [x] 3.4 Delete `audio/playback.rs`; update `audio/mod.rs` to declare `pub mod output;` instead of `pub mod playback;` → `cargo check`

## Phase 4: Dependency Cleanup

- [ ] 4.1 Delete `src-tauri/src/plugins/mod.rs` + `plugins/runtime.rs`
- [ ] 4.2 Delete `src-tauri/src/visualizer/renderer.rs`; update `visualizer/mod.rs` to remove `mod renderer`, add `pub mod fft_bridge;`
- [ ] 4.3 Remove `wasmtime = "19"` and `wgpu = "22"` from `Cargo.toml` [dependencies]
- [ ] 4.4 Remove `mod plugins` from `main.rs` → `cargo check`

## Phase 5: Slim main.rs + Re-export Cleanup

- [ ] 5.1 Slim `main.rs` to only `mod` declarations + `fn main()` with `tauri::Builder`
- [ ] 5.2 Remove all `pub use` backward-compat bridges from `sources/mod.rs` (Track, SourceError)
- [ ] 5.3 Add `#[cfg(test)] mod tests` in `errors/types.rs` with `From<SourceError>` mapping test → `cargo test`