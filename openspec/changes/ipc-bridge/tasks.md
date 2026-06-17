# Tasks: IPC Bridge

## Review Workload Forecast

| Field | Value |
|-------|-------|
| Estimated changed lines | 400–550 |
| 400-line budget risk | Medium |
| Chained PRs recommended | Yes |
| Suggested split | PR 1: Rust backend (playback + ipc) → PR 2: Frontend alignment |
| Delivery strategy | auto-chain |
| Chain strategy | feature-branch-chain |

Decision needed before apply: No
Chained PRs recommended: Yes
Chain strategy: feature-branch-chain
400-line budget risk: Medium

### Suggested Work Units

| Unit | Goal | Likely PR | Notes |
|------|------|-----------|-------|
| 1 | Rust: PlaybackService facade + state + events + commands + setup | PR 1 | Base: feature/ipc-bridge; ~300 lines |
| 2 | Frontend: commands.ts + events.ts alignment | PR 2 | Base: PR 1 branch; ~80 lines |

## Phase 1: Foundation (Playback Domain Types)

- [x] 1.1 Create `src-tauri/src/playback/state.rs` — `PlaybackState` enum (Stopped, Playing, Paused, Buffering with PascalCase serde) and `QueueState` struct (tracks: Vec<Track>, current_index: Option<usize>) with camelCase serde
- [x] 1.2 Create `src-tauri/src/playback/models.rs` — `ProgressTick` struct (position: f64, duration: f64) with camelCase serde
- [x] 1.3 Create `src-tauri/src/playback/events.rs` — `PlaybackEventEmitter` struct wrapping `AppHandle`, with `emit_track_changed`, `emit_state_changed`, `emit_queue_updated`, `emit_progress_tick` methods using `app.emit()`
- [x] 1.4 Update `src-tauri/src/playback/mod.rs` — Export `state`, `models`, `events` modules and re-export key types

## Phase 2: Core Implementation (PlaybackService + Commands)

- [x] 2.1 Create `src-tauri/src/playback/service.rs` — `PlaybackService` struct with `audio`, `queue`, `current_track`, `emitter` fields; method stubs for play, pause, resume, next, previous, seek, set_volume, search, add_to_queue, get_queue; `new()` takes `Box<dyn AudioBackend + Send>` and `AppHandle`
- [x] 2.2 Rewrite `src-tauri/src/ipc/commands.rs` — Define `AppState { playback: Arc<PlaybackService> }`; rewrite all 11 commands to delegate to PlaybackService methods; remove direct AudioBackend access
- [x] 2.3 Rewrite `src-tauri/src/ipc/events.rs` — Define event name constants (`TRACK_CHANGED`, `STATE_CHANGED`, `QUEUE_UPDATED`, `PROGRESS_TICK`); re-export `PlaybackEventEmitter`
- [x] 2.4 Update `src-tauri/src/ipc/mod.rs` — Ensure `commands` and `events` modules are exported

## Phase 3: Integration (App Setup + Main)

- [x] 3.1 Rewrite `src-tauri/src/app/setup.rs` — `build_app()` function returning `tauri::Builder` that registers `AppState(Arc<PlaybackService>)`, all 11 commands in `invoke_handler`, and sets up the Tauri builder
- [x] 3.2 Update `src-tauri/src/main.rs` — Replace inline Tauri builder with `app::setup::build_app()` call; remove `AppState` and `CpalBackend` imports from main
- [x] 3.3 Verify `cargo build` passes with new module structure

## Phase 4: Frontend Alignment

- [x] 4.1 Update `ui/src/services/commands.ts` — Add `resume`, `seek`, `addToQueue`, `getQueue`, `getVersion` functions; fix `play` to accept `url` instead of `trackId`; add `previous` alongside `next`; keep `setVolume`
- [x] 4.2 Update `ui/src/services/events.ts` — Add `ProgressTick` interface; update `onProgressTick` to use `ProgressTick` type instead of `number`

## Phase 5: Testing

- [x] 5.1 Add unit tests in `playback/state.rs` — Verify `PlaybackState` PascalCase serde and `QueueState` camelCase serde
- [x] 5.2 Add unit tests in `playback/models.rs` — Verify `ProgressTick` camelCase serialization round-trip
- [x] 5.3 Add unit tests in `playback/events.rs` — Verify event name constants match expected strings
- [x] 5.4 Add unit tests in `ipc/commands.rs` — Verify `AppState` construction and command function signatures compile correctly
- [x] 5.5 Run `cargo test` — All existing and new tests pass