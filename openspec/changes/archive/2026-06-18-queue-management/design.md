# Design: Queue Management

## Technical Approach

Keep Rust as the source of truth. Add three queue commands in Tauri, implement queue mutation helpers in `PlaybackService`, and let Svelte consume only full `QueueState` snapshots plus existing playback events.

## Architecture Decisions

| Decision | Choice | Alternatives | Rationale |
|---|---|---|---|
| Queue authority | Mutate queue only in `PlaybackService` | Local Svelte mutations | Current architecture is backend-owned; this avoids drift between playback and UI. |
| Snapshot sync | Emit full `queue-updated` after remove/clear/play-next and after `next`/`previous` index changes | Partial diffs | Existing frontend already consumes full `QueueState`; fixing stale UI is cheaper than diff protocols. |
| Index repair | Centralize currentIndex and `played_indices` rebasing in helper methods | Ad hoc logic per command | Remove and insert both can invalidate shuffle history; one helper reduces off-by-one bugs. |

## Data Flow

```text
Queue UI / TrackList / ResultsList
        │ click action
        ▼
ui/src/services/commands.ts
        │ invoke
        ▼
src-tauri/src/ipc/commands.rs
        │ delegate
        ▼
src-tauri/src/playback/service.rs
        │ mutate queue + stop/playback state if needed
        ├── emit state-changed / track-changed when required
        └── emit queue-updated(QueueState)
                ▼
ui/src/features/player/stores/player.ts
        ▼
Queue and list components rerender from snapshot
```

## File Changes

| File | Action | Description |
|---|---|---|
| `src-tauri/src/playback/service.rs` | Modify | Add remove, clear, play-next, snapshot helpers, and queue-updated emission for `next`/`previous`. |
| `src-tauri/src/ipc/commands.rs` | Modify | Expose `remove_from_queue`, `clear_queue`, and `play_next` commands. |
| `ui/src/services/commands.ts` | Modify | Add typed wrappers for the new queue commands. |
| `ui/src/features/player/stores/player.ts` | Modify | Add queue action methods used by UI. |
| `ui/src/features/player/components/Queue.svelte` | Modify | Add remove-per-track and clear-queue controls. |
| `ui/src/shared/utils/actions.ts` | Modify | Add shared play-next action wrapper and queue notifications. |
| `ui/src/shared/components/TrackList.svelte` | Modify | Surface play-next beside add-to-queue. |
| `ui/src/features/search/components/ResultsList.svelte` | Modify | Surface play-next beside add-to-queue. |

## Interfaces / Contracts

```rust
#[tauri::command] fn remove_from_queue(state: State<AppState>, track_id: &str) -> Result<(), AppError>;
#[tauri::command] fn clear_queue(state: State<AppState>) -> Result<(), AppError>;
#[tauri::command] fn play_next(state: State<AppState>, track_id: &str) -> Result<(), AppError>;
```

Queue mutation helpers should return or immediately emit a full `QueueState` snapshot. When a removed index exists in `played_indices`, the helper should drop that entry and shift higher indices down by one.

## Testing Strategy

| Layer | What to Test | Approach |
|---|---|---|
| Unit | Index adjustment and `played_indices` rebasing | Add Rust tests around pure queue helper logic in `service.rs`. |
| Unit | Sequential/shuffle snapshot emission triggers | Extend playback service tests for remove/clear/play-next rules and next/previous queue refresh. |
| UI smoke | Action wiring | Manually verify Queue, TrackList, and ResultsList controls because frontend test infra is not bootstrapped. |

## Migration / Rollout

No migration required.

## Open Questions

- [ ] None.
