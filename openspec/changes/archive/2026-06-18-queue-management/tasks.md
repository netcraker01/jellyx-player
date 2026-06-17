# Tasks: Queue Management

## Review Workload Forecast

| Field | Value |
|---|---|
| Estimated changed lines | 260-340 |
| 400-line budget risk | Low |
| Chained PRs recommended | No |
| Suggested split | Single PR |
| Delivery strategy | ask-always |
| Chain strategy | pending |

Decision needed before apply: Yes
Chained PRs recommended: No
Chain strategy: pending
400-line budget risk: Low

### Suggested Work Units

| Unit | Goal | Likely PR | Notes |
|---|---|---|---|
| 1 | Backend queue mutations + IPC + Svelte wiring | PR 1 | Fits one review slice if UI stays scoped to listed files. |

## Phase 1: Backend Queue Foundation

- [x] 1.1 Add queue helper logic in `src-tauri/src/playback/service.rs` for removing, clearing, inserting-next, and rebasing `current_index` plus `played_indices`.
- [x] 1.2 Update `next()` and `previous()` in `src-tauri/src/playback/service.rs` to emit `queue-updated` after active-index changes.
- [x] 1.3 Expose `remove_from_queue`, `clear_queue`, and `play_next` in `src-tauri/src/ipc/commands.rs`.

## Phase 2: Frontend Command and Store Wiring

- [x] 2.1 Add typed wrappers in `ui/src/services/commands.ts` for remove-from-queue, clear-queue, and play-next.
- [x] 2.2 Add queue action methods and error handling in `ui/src/features/player/stores/player.ts`.
- [x] 2.3 Extend `ui/src/shared/utils/actions.ts` with a shared play-next action and queue mutation notifications.

## Phase 3: UI Controls

- [x] 3.1 Update `ui/src/features/player/components/Queue.svelte` with remove buttons per row and a clear-queue control.
- [x] 3.2 Update `ui/src/shared/components/TrackList.svelte` to expose play-next beside add-to-queue.
- [x] 3.3 Update `ui/src/features/search/components/ResultsList.svelte` to expose play-next beside add-to-queue.

## Phase 4: Verification

- [x] 4.1 Add Rust tests in `src-tauri/src/playback/service.rs` for remove-current, remove-before-current, clear queue, and play-next insertion scenarios.
- [x] 4.2 Add Rust tests in `src-tauri/src/playback/service.rs` for `played_indices` rebasing and queue-updated emission paths covered by navigation changes.
- [x] 4.3 Manually verify Queue, TrackList, and ResultsList flows against spec scenarios because UI automation is unavailable.
