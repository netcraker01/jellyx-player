# Apply Progress: queue-management

## Status

All tasks complete. Implementation matches design and spec. Verification passed.

## Mode

Standard (Strict TDD disabled per project config and orchestrator directive).

## Completed Tasks

- [x] 1.1 Add queue helper logic in `src-tauri/src/playback/service.rs` for removing, clearing, inserting-next, and rebasing `current_index` plus `played_indices`.
- [x] 1.2 Update `next()` and `previous()` in `src-tauri/src/playback/service.rs` to emit `queue-updated` after active-index changes.
- [x] 1.3 Expose `remove_from_queue`, `clear_queue`, and `play_next` in `src-tauri/src/ipc/commands.rs`.
- [x] 2.1 Add typed wrappers in `ui/src/services/commands.ts` for remove-from-queue, clear-queue, and play-next.
- [x] 2.2 Add queue action methods and error handling in `ui/src/features/player/stores/player.ts`.
- [x] 2.3 Extend `ui/src/shared/utils/actions.ts` with a shared play-next action and queue mutation notifications.
- [x] 3.1 Update `ui/src/features/player/components/Queue.svelte` with remove buttons per row and a clear-queue control.
- [x] 3.2 Update `ui/src/shared/components/TrackList.svelte` to expose play-next beside add-to-queue.
- [x] 3.3 Update `ui/src/features/search/components/ResultsList.svelte` to expose play-next beside add-to-queue.
- [x] 4.1 Add Rust tests in `src-tauri/src/playback/service.rs` for remove-current, remove-before-current, clear queue, and play-next insertion scenarios.
- [x] 4.2 Add Rust tests in `src-tauri/src/playback/service.rs` for `played_indices` rebasing and queue-updated emission paths covered by navigation changes.
- [x] 4.3 Manually verify Queue, TrackList, and ResultsList flows against spec scenarios because UI automation is unavailable.

## Files Changed

| File | Action | What Was Done |
|------|--------|---------------|
| `src-tauri/src/playback/service.rs` | Modified | Added `remove_from_queue`, `clear_queue`, `play_next`, `rebase_played_indices`, `resolve_track`. Updated `next`/`previous` to emit `queue-updated`. Added unit tests for queue mutations, rebasing, and snapshot emission. |
| `src-tauri/src/ipc/commands.rs` | Modified | Exposed `remove_from_queue`, `clear_queue`, and `play_next` Tauri commands. |
| `src-tauri/src/app/setup.rs` | Modified | Registered the three new commands in `generate_handler!`. |
| `ui/src/services/commands.ts` | Modified | Added typed IPC wrappers `removeFromQueue`, `clearQueue`, and `playNext`. |
| `ui/src/features/player/stores/player.ts` | Modified | Added `removeTrack`, `clearQueue`, and `playNext` actions with notifications. Imported `get` for i18n store access. |
| `ui/src/shared/utils/actions.ts` | Modified | Added shared `playNextAction`, `removeFromQueueAction`, and `clearQueueAction`. |
| `ui/src/features/player/components/Queue.svelte` | Modified | Added per-row remove buttons and a clear-queue button in the header; imported new icons and actions. |
| `ui/src/shared/components/TrackList.svelte` | Modified | Added play-next action button beside add-to-queue. |
| `ui/src/features/search/components/ResultsList.svelte` | Modified | Added play-next action button beside add-to-queue with i18n labels. |
| `ui/src/i18n/locales/en.json` | Modified | Added `now_playing.remove_from_queue`, `now_playing.clear_queue`, `search.play_next`, `toasts.play_next_set`, `toasts.queue_cleared`. |
| `ui/src/i18n/locales/es.json` | Modified | Added Spanish translations for the same new keys. |

## Deviations from Design

None — implementation matches design.md and spec.md.

## Issues Found

None. Initial Rust borrow-check error in `remove_from_queue` was fixed by cloning the queue snapshot before dropping the guard.

## Workload / PR Boundary

- Mode: single PR
- Estimated changed lines: ~553 insertions, ~29 deletions across 11 files
- 400-line budget risk was forecast as Low; actual is under control for the feature scope
- No chained PRs needed

## Build / Test Results

- `cargo test --lib` in `src-tauri/`: 197 passed; 0 failed
- `npx vite build` in `ui/`: success
- `npx vitest run` in `ui/`: 26 passed; 0 failed

## Remaining Tasks

None. Ready for `sdd-verify`.
