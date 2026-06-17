# Tasks: Playback UX

## Review Workload Forecast

| Field | Value |
|-------|-------|
| Estimated changed lines | 450-700 |
| 400-line budget risk | High |
| Chained PRs recommended | Yes |
| Suggested split | PR 1 backend foundation -> PR 2 frontend wiring |
| Delivery strategy | ask-on-risk |
| Chain strategy | feature-branch-chain |

Decision needed before apply: Resolved (feature-branch-chain)
Chained PRs recommended: Yes
Chain strategy: feature-branch-chain
400-line budget risk: High

### Suggested Work Units

| Unit | Goal | Likely PR | Notes |
|------|------|-----------|-------|
| 1 | Backend queue/history/favorite contract | PR 1 | Rust only; includes queue snapshot, IPC, DB cap, tests/docs |
| 2 | Frontend playback/favorite controls | PR 2 | Base after PR 1; store sync, controls, locales, UX verification |

## Phase 1: Backend Foundation

- [x] 1.1 Update `src-tauri/src/playback/state.rs` with `RepeatMode`, `shuffle`, and `played_indices` on `QueueState`.
- [x] 1.2 Update `src-tauri/src/playback/events.rs` so `emit_queue_updated` emits full `&QueueState` snapshots.
- [x] 1.3 Update `src-tauri/src/app/setup.rs` and `src-tauri/src/playback/service.rs` to inject `Arc<LibraryService>` into `PlaybackService`.
- [x] 1.4 Update `src-tauri/src/persistence/db.rs` with `favorite_exists()` and 100-entry history eviction in `insert_history()`.

## Phase 2: Backend Behavior

- [x] 2.1 Update `src-tauri/src/library/service.rs` with atomic `toggle_favorite()` and bounded history reads.
- [x] 2.2 Update `src-tauri/src/ipc/commands.rs` with `toggle_favorite`, `set_shuffle`, `set_repeat`, and `cycle_repeat` commands.
- [x] 2.3 Update `src-tauri/src/playback/service.rs` to record history exactly once in `play_local()` after Playing state.
- [x] 2.4 Update `src-tauri/src/playback/service.rs` next/end-of-track logic for shuffle-without-reorder and repeat off/all/one.

## Phase 3: Frontend Wiring

- [x] 3.1 Update `ui/src/features/player/stores/player.ts` to consume queue snapshots and expose shuffle/repeat/currentIndex state.
- [x] 3.2 Update `ui/src/features/favorites/stores/favorites.ts` to add `toggle()` backed by `toggle_favorite`.
- [x] 3.3 Update `ui/src/features/player/components/NowPlayingInfo.svelte` with persisted heart toggle for the current track.
- [x] 3.4 Update `ui/src/features/player/components/Controls.svelte`, `ui/src/app/layout/BottomBar.svelte`, `ui/src/i18n/locales/en.json`, and `ui/src/i18n/locales/es.json` with shuffle/repeat controls and labels.

## Phase 4: Verification

- [x] 4.1 Verify Rust behavior for history-on-start-only, oldest-entry eviction at 101st play, and favorite toggle uniqueness.
- [x] 4.2 Verify playback modes: shuffle picks remaining unplayed tracks without reordering, repeat-all loops, repeat-one replays.
- [x] 4.3 Verify UI integration: Home shows recent plays, Now Playing restores favorite state, queue controls stay in sync with backend events.
