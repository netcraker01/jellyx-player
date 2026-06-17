# Proposal: IPC Bridge

## Intent

The 7 existing Tauri commands bypass PlaybackService, event emissions are placeholder-only, and frontend/Rust command signatures are misaligned. This change wires IPC commands through PlaybackService, implements real event emissions, and aligns all signatures — establishing the architecture mandated by ARCHITECTURE.md §1–2 where Rust is Source of Truth and Svelte is a "dumb client."

## Scope

### In Scope
- PlaybackService facade with method stubs (play, pause, next, previous, volume, search, add_to_queue, get_queue)
- Rewrite `ipc/commands.rs` to route through PlaybackService with real types (Track, PlaybackState)
- Implement `ipc/events.rs` with 4 typed event emission helpers (track_changed, state_changed, queue_updated, progress_tick)
- Implement `playback/state.rs` (PlaybackState enum + queue) and `playback/events.rs` (event payload structs)
- Implement `playback/service.rs` as facade wrapping AudioBackend
- Update `app/setup.rs` with Tauri builder wiring all commands + AppHandle for events
- Update `main.rs` to use setup module
- Align `ui/src/services/commands.ts` with Rust command signatures (add resume, seek, get_version; fix play signature)
- Update `ui/src/services/events.ts` with typed event payloads (ProgressTick struct)
- camelCase parameter naming convention enforced on all command boundaries

### Out of Scope
- Binary FFT bridge (visualizer/fft_bridge.rs) — deferred to future change
- Real audio playback integration inside PlaybackService (facade with stubs only)
- Library/persistence features
- Source resolver improvements (YouTubeResolver stays as-is)

## Capabilities

### New Capabilities
- `ipc-commands`: Command routing through PlaybackService — all Tauri commands delegate to PlaybackService methods with real types
- `ipc-events`: Event emission from Rust to Svelte — typed event payloads for track_changed, state_changed, queue_updated, progress_tick

### Modified Capabilities
- None (no existing specs to modify)

## Approach

**Approach 2 from exploration**: IPC Commands + Events First, Binary FFT Deferred.

- PlaybackService exists as a facade with method stubs — commands delegate to it, not directly to AudioBackend
- Event emissions use Tauri v2 `AppHandle.emit()` with typed Rust structs serialized via serde (camelCase)
- Frontend commands.ts aligns with Rust: `play(url)` → `play(url)` matching Rust, add missing `resume`, `seek`, `get_version`
- Frontend events.ts adds `ProgressTick` type for typed payloads
- `app/setup.rs` centralizes Tauri builder config, extracting it from `main.rs`

## Affected Areas

| Area | Impact | Description |
|------|--------|-------------|
| `src-tauri/src/ipc/commands.rs` | Modified | Rewrite to use PlaybackService + real types |
| `src-tauri/src/ipc/events.rs` | Modified | Implement 4 event emission helpers |
| `src-tauri/src/ipc/mod.rs` | Modified | Update module exports |
| `src-tauri/src/playback/service.rs` | Modified | PlaybackService facade with stubs |
| `src-tauri/src/playback/state.rs` | Modified | PlaybackState + queue struct |
| `src-tauri/src/playback/events.rs` | Modified | Event payload structs |
| `src-tauri/src/playback/models.rs` | Modified | Internal DTOs |
| `src-tauri/src/app/setup.rs` | Modified | Tauri builder wiring |
| `src-tauri/src/main.rs` | Modified | Use setup module |
| `ui/src/services/commands.ts` | Modified | Align signatures, add missing commands |
| `ui/src/services/events.ts` | Modified | Add ProgressTick type |

## Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| PlaybackService facade has no real playback | Low | Expected — facade pattern, implementation in future change |
| Tauri v2 event API differences from v1 | Med | Verify `AppHandle.emit()` vs `Window.emit()` in Tauri v2 docs |
| Command param naming mismatch | Med | Enforce camelCase convention; Tauri requires exact match |
| AppState grows with PlaybackService | Low | Use Arc<PlaybackService> in AppState, not nested Mutex |

## Rollback Plan

Revert all changed files. The existing commands.rs pattern (direct AudioBackend access) still compiles. `git revert` on the change commit restores working state.

## Dependencies

- Changes #1–#3 (scaffold, frontend, models-and-errors) must be complete ✓
- Tauri v2 crate with `event` feature enabled (verify Cargo.toml)

## Success Criteria

- [ ] All Tauri commands route through PlaybackService (no direct AudioBackend access from commands)
- [ ] 4 event types emit from Rust with typed payloads
- [ ] Frontend command signatures match Rust 1:1 (camelCase)
- [ ] `cargo build` and `cargo test` pass
- [ ] `app/setup.rs` contains Tauri builder config, not `main.rs`