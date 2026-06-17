# Proposal: Playback UX

## Intent

Deliver quick-win playback UX: make Home's recently played actually populate, add a persistent favorite heart in Now Playing, and add backend-driven shuffle/repeat behavior that matches Helix's Rust-owned playback architecture.

## Scope

### In Scope
- Record one history entry when playback starts for local, YouTube, and SoundCloud tracks; keep only the latest 100 entries.
- Add a Now Playing favorite toggle that adds/removes the current track and stays correct across sessions.
- Add shuffle and repeat modes to queue state and controls: shuffle picks the next remaining track without visually reordering the queue; repeat cycles off → all → one.

### Out of Scope
- Share action, drag-to-reorder queue, artist/album detail views.
- Search grouped by type and any visual queue shuffle.

## Capabilities

### New Capabilities
- `play-history`: Record playback starts across all sources, expose bounded recently played data, and preserve Home behavior.
- `favorites-management`: Support toggle-style favorite actions from Now Playing with persisted favorite state.
- `playback-modes`: Expose backend-owned queue snapshot, shuffle-next logic, and repeat-all/repeat-one behavior.

### Modified Capabilities
- None.

## Approach

Use backend-owned state. Extend `QueueState` and queue events/IPC to return a queue snapshot with `tracks`, `currentIndex`, shuffle, and repeat mode. Add `toggle_favorite` IPC backed by existing SQLite favorites persistence. Inject history recording at real playback start in `PlaybackService`, so `next()`/`previous()` inherit the rule automatically.

## Affected Areas

| Area | Impact | Description |
|------|--------|-------------|
| `src-tauri/src/playback/service.rs` | Modified | Record history on track start; apply shuffle/repeat next-track rules. |
| `src-tauri/src/playback/state.rs`, `src-tauri/src/playback/events.rs`, `src-tauri/src/ipc/commands.rs` | Modified | Add queue snapshot + playback mode/toggle commands. |
| `src-tauri/src/library/service.rs` | Modified | Add favorite toggle service path using existing add/remove persistence. |
| `ui/src/features/player/**/*`, `ui/src/app/layout/BottomBar.svelte`, `ui/src/routes/Home/Page.svelte` | Modified | Surface favorite, shuffle, repeat, and recently played UX. |

## Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| Queue/UI state drift during contract change | Med | Keep Rust as source of truth and migrate UI to one queue snapshot payload. |
| History duplicates from seek/resume paths | Med | Record only on track-start entry points, not seek/resume handlers. |
| Stale favorite heart state | Low | Load favorites early and update via single toggle flow. |

## Rollback Plan

Revert queue snapshot/toggle IPC changes and restore current `Track[]` queue contract; disable history hook if duplicate or cross-source regressions appear.

## Dependencies

- Existing SQLite favorites/history tables and current Tauri IPC/event infrastructure.

## Success Criteria

- [ ] Playing any source adds one recent item on Home, capped at 100 entries.
- [ ] Heart button in Now Playing toggles persisted favorite state without duplicates.
- [ ] Shuffle changes only next-track selection; repeat cycles off/all/one and behaves correctly at track end.
