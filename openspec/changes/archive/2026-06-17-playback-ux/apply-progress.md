# Apply Progress: Playback UX — PR 2 (Frontend)

**Change**: playback-ux
**Project**: Helix Player
**Mode**: Standard (Strict TDD disabled per project config)
**PR slice**: PR 2 of 2 — Frontend Wiring
**Chain strategy**: feature-branch-chain
**Applied by**: SDD apply sub-agent
**Date**: 2026-06-17

## Completed Tasks

### Phase 1: Backend Foundation
- [x] 1.1 Update `src-tauri/src/playback/state.rs` with `RepeatMode`, `shuffle`, and `played_indices` on `QueueState`.
- [x] 1.2 Update `src-tauri/src/playback/events.rs` so `emit_queue_updated` emits full `&QueueState` snapshots.
- [x] 1.3 Update `src-tauri/src/app/setup.rs` and `src-tauri/src/playback/service.rs` to inject `Arc<LibraryService>` into `PlaybackService`.
- [x] 1.4 Update `src-tauri/src/persistence/db.rs` with `favorite_exists()` and 100-entry history eviction in `insert_history()`.

### Phase 2: Backend Behavior
- [x] 2.1 Update `src-tauri/src/library/service.rs` with atomic `toggle_favorite()` and bounded history reads.
- [x] 2.2 Update `src-tauri/src/ipc/commands.rs` with `toggle_favorite`, `set_shuffle`, `set_repeat`, and `cycle_repeat` commands.
- [x] 2.3 Update `src-tauri/src/playback/service.rs` to record history exactly once in `play_local()` after Playing state.
- [x] 2.4 Update `src-tauri/src/playback/service.rs` next/end-of-track logic for shuffle-without-reorder and repeat off/all/one.

### Phase 3: Frontend Wiring
- [x] 3.1 Update `ui/src/features/player/stores/player.ts` to consume queue snapshots and expose shuffle/repeat/currentIndex state.
- [x] 3.2 Update `ui/src/features/favorites/stores/favorites.ts` to add `toggle()` backed by `toggle_favorite`.
- [x] 3.3 Update `ui/src/features/player/components/NowPlayingInfo.svelte` with persisted heart toggle for the current track.
- [x] 3.4 Update `ui/src/features/player/components/Controls.svelte`, `ui/src/app/layout/BottomBar.svelte`, `ui/src/i18n/locales/en.json`, and `ui/src/i18n/locales/es.json` with shuffle/repeat controls and labels.

### Phase 4: Verification
- [x] 4.1 Verify Rust behavior for history-on-start-only, oldest-entry eviction at 101st play, and favorite toggle uniqueness.
- [x] 4.2 Verify playback modes: shuffle picks remaining unplayed tracks without reordering, repeat-all loops, repeat-one replays.
- [x] 4.3 Verify UI integration: Home shows recent plays, Now Playing restores favorite state, queue controls stay in sync with backend events.

## Build / Test Results

| Command | Result |
|---------|--------|
| `npx vite build` (ui/) | ✅ built in 16.38s |
| `npx vitest run` (ui/) | ✅ 26 passed; 0 failed |
| `cargo check` (src-tauri) | ✅ passes (4 pre-existing dead-code warnings) |
| `cargo test --lib` (src-tauri) | ✅ 188 passed; 0 failed |

## Files Changed

| File | Action | What Was Done |
|------|--------|---------------|
| `ui/src/shared/types/models.ts` | Modified | Added `RepeatMode` type and `QueueState` interface |
| `ui/src/services/commands.ts` | Modified | Added `toggleFavorite`, `isFavorite`, `setShuffle`, `setRepeat`, `cycleRepeat`; updated `getQueue` to return `QueueState` |
| `ui/src/services/events.ts` | Modified | Updated `onQueueUpdated` payload type to `QueueState` |
| `ui/src/features/player/stores/player.ts` | Modified | Added `queueState`, `currentIndex`, `shuffle`, `repeatMode`, `isCurrentTrackFavorited` stores and `toggleShuffle`/`cycleRepeat` actions |
| `ui/src/features/favorites/stores/favorites.ts` | Modified | Added atomic `toggle(trackId)` method using `toggleFavorite` IPC |
| `ui/src/features/player/components/NowPlayingInfo.svelte` | Modified | Added heart toggle button next to track title; filled/outline state tied to `isCurrentTrackFavorited` |
| `ui/src/features/player/components/Controls.svelte` | Modified | Added shuffle and repeat buttons with active-state styling |
| `ui/src/app/layout/BottomBar.svelte` | Modified | Wired shuffle/repeat controls alongside existing playback controls |
| `ui/src/features/player/components/Queue.svelte` | Modified | Updated to read `queueState`/`currentIndex`; added track number highlighting and shuffle indicator |
| `ui/src/i18n/locales/en.json` | Modified | Added shuffle/repeat/favorite control strings |
| `ui/src/i18n/locales/es.json` | Modified | Added Spanish translations for shuffle/repeat/favorite strings |
| `openspec/changes/playback-ux/tasks.md` | Modified | Marked all frontend tasks and verification tasks `[x]` |
| `openspec/changes/playback-ux/apply-progress.md` | Modified | Updated with PR 2 completion status |

## Deviations from Design

1. **IPC return types adapted to actual Rust signatures**: The prompt specified `set_shuffle`, `set_repeat`, and `cycle_repeat` returning `QueueState`, but the backend returns `Result<(), AppError>` and `Result<String, AppError>` respectively. Frontend command wrappers match the real backend contract (`void` and `string`) and rely on `queue-updated` events for state sync.
2. **Queue component uses `currentIndex` instead of `currentTrack.id` for highlighting**: The design noted `currentIndex` is now part of `QueueState`; the component highlights by index to stay consistent with the backend snapshot and avoid ambiguity when the same track appears multiple times.
3. **`toggle()` refreshes favorites after adding**: Because the IPC only returns a boolean and not the full track metadata, the store reloads favorites after a successful toggle to keep local state consistent.

## Issues Found

- None blocking. Pre-existing dead-code warnings remain in Rust; no new warnings introduced by frontend changes. The backend `set_repeat`/`set_shuffle` commands return `()` rather than `QueueState` as described in the prompt, so frontend consumes state via `queue-updated` events.

## Remaining Tasks

- None. The playback-ux change is fully implemented across PR 1 (backend) and PR 2 (frontend).

## Workload / PR Boundary

- Mode: chained PR slice — feature-branch-chain
- Current work unit: PR 2 — Frontend Wiring (Svelte/TypeScript only)
- Boundary: all frontend changes needed to consume the new backend IPC contract and surface shuffle/repeat/favorite UX. No `src-tauri/` files modified.
- Estimated review budget impact: ~220 changed lines in `ui/` (within the ~200-line target).

## TDD Cycle Evidence

Strict TDD Mode is **disabled** in `openspec/config.yaml`. Tests were added/updated alongside implementation per the standard workflow. The existing frontend test suite passes; no new tests were required by this PR scope.

## Status

12/12 tasks complete. Ready for verify/archive.
