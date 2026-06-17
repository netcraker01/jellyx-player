# Design: Playback UX

## Technical Approach

Extend the Rust backend as source of truth for playback modes and history recording. Add a `toggle_favorite` IPC command. Surface new controls in the Svelte frontend.

## Architecture Decisions

### Decision 1: Inject LibraryService into PlaybackService for history recording

**Context**: `PlaybackService` needs to call `LibraryService::record_play()` when a track starts playing, but currently has no reference to it.

**Choice**: Add `Arc<LibraryService>` as a field of `PlaybackService`. Inject it in `PlaybackService::new()`.

**Alternatives considered**:
- Callback/event-based: More decoupled but adds complexity for a simple call.
- Frontend-triggered IPC: Race conditions — frontend might miss track changes or double-record on reconnection.

**Rationale**: Backend-owned state means backend-owned side effects. The playback service is the single point where tracks start, so it's the correct place to record history.

### Decision 2: Extend QueueState with shuffle and repeat mode fields

**Context**: `QueueState` currently has only `tracks: Vec<Track>` and `current_index: Option<usize>`. Shuffle and repeat need to be part of the source of truth.

**Choice**: Add to `QueueState`:
```rust
pub repeat_mode: RepeatMode,
pub shuffle: bool,
pub played_indices: Vec<usize>,  // tracks already played during shuffle
```

Where:
```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum RepeatMode {
    Off,
    All,
    One,
}
```

**Alternatives considered**:
- Separate struct for modes: More types but same data, no real benefit.
- Frontend-only mode state: Violates backend-owned architecture.

**Rationale**: QueueState already represents the queue truth. Modes are queue behavior, so they belong together.

### Decision 3: Shuffle selects next randomly without reordering queue

**Context**: User specified shuffle should NOT visually reorder the queue. It should only decide the next track randomly.

**Choice**: Maintain `played_indices: Vec<usize>` in QueueState. When shuffle is on and next is requested:
1. Build a list of unplayed indices: `0..tracks.len()` minus `played_indices` minus `current_index`.
2. Pick a random index from that list.
3. Set `current_index` to the picked index and add it to `played_indices`.
4. The `tracks` Vec stays in original order.

When all tracks are played and repeat is off, playback stops.
When all tracks are played and repeat-all, clear `played_indices` and restart.

**Rationale**: Matches user requirement exactly. Visual queue stays stable. Simple implementation.

### Decision 4: Rich queue snapshot event replaces plain track array

**Context**: `emit_queue_updated()` currently sends `Vec<Track>`. Frontend needs current_index, shuffle, and repeat state.

**Choice**: Replace the `queue-updated` event payload with `QueueState` (already serializable). The frontend subscribes to a single event and gets the full snapshot.

**Alternatives considered**:
- Separate events for mode changes: More events, more state sync issues.
- Polling: Defeats the purpose of events.

**Rationale**: Single source of truth, single event to subscribe to.

### Decision 5: `toggle_favorite` IPC command on backend

**Context**: Frontend currently calls separate `add_favorite` / `remove_favorite`. A toggle needs to check current state first.

**Choice**: Add `toggle_favorite(track_id: String)` IPC command that:
1. Checks `db.favorite_exists(track_id)`.
2. If exists → remove. If not → add.
3. Returns the new state (`true` = favorited, `false` = not favorited).

**Alternatives considered**:
- Frontend composition of add/remove: Requires two round-trips or frontend state tracking, which can go stale.
- Frontend checks and then calls add/remove: Race condition between check and action.

**Rationale**: Atomic toggle is simpler and avoids stale state. One IPC call instead of two.

### Decision 6: Record history exactly once per track start

**Context**: `next()`, `previous()`, and `play()` all eventually call `play_local()`. History should be recorded once when a track starts, not on every seek or resume.

**Choice**: Add `record_history(&self, track: &Track)` as a private method on `PlaybackService` that calls `self.library.record_play(track)`. Call it in `play_local()` right after state is updated to `Playing`, before starting the decoder thread. Do NOT call it on seek, resume, or volume changes.

**Rationale**: `play_local()` is the single entry point where local playback actually starts. `next()` and `previous()` delegate to `play_local()` for local tracks, so they inherit history recording.

## Data Flow

### History Recording
```
User plays track → PlaybackService::play_local()
    → update state to Playing
    → self.record_history(&track)  // calls LibraryService::record_play()
    → LibraryService::record_play() → Database::insert_history()
    → Database evicts oldest if > 100 entries
    → emit track-changed event
```

### Favorite Toggle
```
User clicks heart → frontend calls toggle_favorite(track_id)
    → IPC command checks db.favorite_exists(track_id)
    → if exists: db.remove_favorite(track_id), return false
    → if not exists: db.insert_favorite(track), return true
    → frontend updates heart icon based on return value
```

### Shuffle Next
```
Track ends or user clicks next → PlaybackService::next()
    → if shuffle mode on:
        → build unplayed = all indices - played_indices - current_index
        → if unplayed.is_empty():
            → if repeat_all: clear played_indices, restart from 0
            → if repeat_off: stop playback
        → pick random from unplayed
        → set current_index = picked
        → add picked to played_indices
    → if shuffle mode off:
        → current_index += 1 (with repeat logic)
    → play_local() or play_remote() based on source
    → record_history() inherited from play_local()
```

### Repeat at Track End
```
Decoder thread hits end of stream
    → if repeat_one: replay current track (same index, call play_local again)
    → if repeat_all and current is last: wrap to index 0
    → if repeat_off and current is last: stop
```

## Files Affected

| File | Action | Description |
|------|--------|-------------|
| `src-tauri/src/playback/state.rs` | Modified | Add `RepeatMode` enum, `shuffle`, `played_indices` to QueueState |
| `src-tauri/src/playback/service.rs` | Modified | Inject LibraryService, add record_history(), shuffle_next(), repeat logic |
| `src-tauri/src/playback/events.rs` | Modified | Change `emit_queue_updated` to take `&QueueState` instead of `&[Track]` |
| `src-tauri/src/ipc/commands.rs` | Modified | Add toggle_favorite, set_shuffle, set_repeat, cycle_repeat commands |
| `src-tauri/src/library/service.rs` | Modified | Add toggle_favorite() method, update get_history() to cap at 100 |
| `src-tauri/src/persistence/db.rs` | Modified | Add favorite_exists(), update history insert to evict at 100 |
| `src-tauri/src/app/setup.rs` | Modified | Pass LibraryService to PlaybackService constructor |
| `ui/src/features/player/stores/player.ts` | Modified | Add shuffle, repeat, queue index state; listen to richer queue events |
| `ui/src/features/favorites/stores/favorites.ts` | Modified | Add toggle() method using toggle_favorite IPC |
| `ui/src/features/player/components/NowPlayingInfo.svelte` | Modified | Add heart/toggle button |
| `ui/src/features/player/components/Controls.svelte` | Modified | Add shuffle and repeat buttons |
| `ui/src/app/layout/BottomBar.svelte` | Modified | Add shuffle/repeat controls to bottom bar |
| `ui/src/routes/Home/Page.svelte` | Modified | No changes needed — already reads get_history() |
| `ui/src/i18n/locales/en.json` | Modified | Add shuffle/repeat/favorite strings |
| `ui/src/i18n/locales/es.json` | Modified | Add shuffle/repeat/favorite strings |

## Testing Strategy

- **Unit (Rust)**: RepeatMode cycle, shuffle index selection, history eviction at 100, toggle_favorite add/remove/duplicate
- **Unit (Rust)**: QueueState serialization with new fields (camelCase)
- **Integration**: Play local file → verify history entry; toggle favorite → verify state; shuffle next → verify random selection