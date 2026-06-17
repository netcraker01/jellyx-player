## Exploration: queue-management

### Current State
Queue state is backend-owned in `PlaybackService`/`QueueState`, but only `add_to_queue`, `get_queue`, `next`, and `previous` exist. There is no remove, clear, or insert-next command. `Queue.svelte` is read-only apart from a play button. The frontend store mirrors `queue-updated` snapshots, but `next()`/`previous()` do not emit `queue-updated`, so `currentIndex` can drift in the UI. Also, queue/local play buttons call `commands.play()`; there is no frontend `play_local` wrapper, so local queue item playback is already inconsistent.

### Affected Areas
- `src-tauri/src/playback/service.rs` — add queue mutation methods and normalize index/playback transitions
- `src-tauri/src/ipc/commands.rs` — expose new Tauri commands for remove, clear, and play-next
- `src-tauri/src/playback/events.rs` — reuse `queue-updated`; likely emit after index-changing navigation too
- `ui/src/services/commands.ts` — add typed wrappers for new IPC commands
- `ui/src/features/player/stores/player.ts` — add actions and keep queue state synced after mutations
- `ui/src/features/player/components/Queue.svelte` — add remove/clear controls and empty-state behavior
- `ui/src/shared/components/TrackList.svelte` — likely add a “Play next” action where “Add to queue” already exists
- `ui/src/features/search/components/ResultsList.svelte` — same play-next entry point for search results

### Approaches
1. **Backend-owned queue mutations** — add Rust queue APIs, emit updated snapshots, keep Svelte as a thin client.
   - Pros: Matches architecture; one source of truth; handles current index/shuffle/repeat safely.
   - Cons: Touches Rust + IPC + multiple UI entry points.
   - Effort: Medium

2. **Frontend-local queue editing** — mutate Svelte stores first, patch backend later.
   - Pros: Faster UI work initially.
   - Cons: Violates architecture; risks desync with playback state/events; harder to preserve repeat/shuffle invariants.
   - Effort: Medium

### Recommendation
Use **Backend-owned queue mutations**. Implement `remove_from_queue`, `clear_queue`, and `play_next` in Rust, define current-track removal rules there, and emit full `QueueState` snapshots after every queue/index mutation.

### Risks
- Removing the current track needs explicit semantics: advance to the next track, fall back to previous when removing the last item, and stop when the queue becomes empty.
- `play_next` is only clean if it inserts after `currentIndex` (not absolute index 0) and updates shuffle bookkeeping.
- Existing local playback wiring is inconsistent (`play_local` missing on the frontend), which can confuse queue action validation.

### Ready for Proposal
Yes — propose backend-first queue management with remove, clear, and play-next in scope, drag-to-reorder deferred, and queue snapshot sync fixed as an implementation requirement.
