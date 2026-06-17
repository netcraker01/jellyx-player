# Proposal: Queue Management

## Intent

Make the queue actionable for MVP by adding remove-track, clear-queue, and play-next behavior required by the PRD, while keeping Rust as the source of truth.

## Scope

### In Scope
- Remove an individual track from the queue.
- Clear the full queue and reset playback when nothing remains.
- Add “Play next” so a selected track is inserted immediately after the current track.
- Keep frontend queue state synced after queue/index mutations.

### Out of Scope
- Drag-to-reorder/manual reordering.
- Share actions, artist views, album views.
- Broader playback fixes unrelated to queue mutation UI.

## Capabilities

### New Capabilities
- `queue-management`: queue mutation behaviors for remove, clear, and play-next, including current-index rules and queue snapshot sync.

### Modified Capabilities
- None.

## Proposal question round

No blocking product questions. Assumption: removing the current track advances to the next available track, or stops playback if the queue becomes empty.

## Approach

Add backend commands for `remove_from_queue`, `clear_queue`, and `play_next`; centralize index adjustment in `PlaybackService`; emit `queue-updated` after every queue-affecting or index-affecting change. Then add typed command wrappers, store actions, remove/clear UI in `Queue.svelte`, and play-next actions beside existing add-to-queue controls.

## Affected Areas

| Area | Impact | Description |
|------|--------|-------------|
| `src-tauri/src/playback/service.rs` | Modified | Queue mutation rules, playback transition handling |
| `src-tauri/src/ipc/commands.rs` | Modified | New Tauri queue commands |
| `ui/src/services/commands.ts` | Modified | Frontend IPC wrappers |
| `ui/src/features/player/stores/player.ts` | Modified | Queue action helpers + sync |
| `ui/src/features/player/components/Queue.svelte` | Modified | Remove and clear controls |
| `ui/src/shared/components/TrackList.svelte` | Modified | Play-next action entry point |
| `ui/src/features/search/components/ResultsList.svelte` | Modified | Play-next action entry point |

## Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| Current index becomes stale after navigation | Med | Emit `queue-updated` on next/previous and queue mutations |
| Current-track removal semantics feel wrong | Med | Specify exact fallback rules in spec before apply |
| Local-track action paths are already inconsistent | Med | Keep queue mutation work independent; flag playback wrapper follow-up if needed |

## Rollback Plan

Revert the new queue IPC commands and UI controls, returning queue interactions to read-only `add_to_queue` + display behavior.

## Dependencies

- Existing `QueueState` snapshot/event contract from playback-ux.

## Success Criteria

- [ ] Users can remove one queued track without breaking queue state.
- [ ] Users can clear the queue and playback stops cleanly when appropriate.
- [ ] Users can mark a track to play next from existing list/search entry points.
- [ ] Queue highlight/order remains in sync after queue and navigation changes.
