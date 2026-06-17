## Exploration: playback-ux

### Current State
Playback is backend-driven from `src-tauri/src/playback/service.rs`, not `src-tauri/src/audio/playback.rs` (that file does not exist). `play_local()` sets `current_track`, emits `track-changed`, then flips state to `Playing`; `next()`/`previous()` only fully start playback for local tracks by delegating to `play_local()`. Play history persistence already exists in `src-tauri/src/library/service.rs` and `src-tauri/src/persistence/db.rs`, and Home reads it via `get_history()`, but no playback path calls `record_play()`. On the frontend, `ui/src/features/player/stores/player.ts` only mirrors `currentTrack`, `isPlaying`, `progress`, `queue`, `volume`, and FFT data; it does not track queue index, shuffle, repeat, or favorite state. Favorites are loaded only when the Favorites route mounts, and Now Playing has no heart action. Queue UI is read-only beyond per-track play, and current IPC only exposes `add_to_queue`/`get_queue` with a plain `Vec<Track>`, so there is no backend or event contract for shuffle/repeat state.

### Affected Areas
- `src-tauri/src/playback/service.rs` — exact history hook point; queue model currently lacks shuffle/repeat behavior and state.
- `src-tauri/src/ipc/commands.rs` — current IPC exposes history/favorites reads and queue commands, but no favorite toggle or shuffle/repeat commands.
- `src-tauri/src/playback/state.rs` — `QueueState` only has `tracks` and `current_index`; missing playback mode fields.
- `src-tauri/src/playback/events.rs` — queue event currently emits only `Track[]`; insufficient for shuffle/repeat/index metadata.
- `src-tauri/src/library/service.rs` — `record_play()` already works and should remain the persistence boundary.
- `ui/src/features/player/stores/player.ts` — frontend store needs queue mode state and control actions; currently only listens to basic events.
- `ui/src/features/player/components/NowPlayingInfo.svelte` — best integration point for favorite toggle beside track metadata.
- `ui/src/routes/NowPlaying/Page.svelte` — page layout decides whether favorite/mode controls live in info block or controls section.
- `ui/src/app/layout/BottomBar.svelte` and `ui/src/features/player/components/Controls.svelte` — existing transport controls layout to extend with shuffle/repeat.
- `ui/src/features/favorites/stores/favorites.ts` — current add/remove flow can be refactored into a toggle path and reused by Now Playing.
- `ui/src/features/player/components/Queue.svelte` — queue display can reflect active order/mode but currently has no mode awareness.
- `ui/src/i18n/locales/en.json` / `ui/src/i18n/locales/es.json` — missing strings for favorite toggle, shuffle, repeat, and mode toasts.

### Approaches
1. **Backend-owned playback modes** — extend Rust queue state, IPC, and events so shuffle/repeat live in the source of truth.
   - Pros: Matches project architecture; history can be recorded exactly when playback truly starts; shuffle/repeat stays consistent across BottomBar, Now Playing, and queue navigation.
   - Cons: Requires widening IPC/event contracts and frontend store shape.
   - Effort: Medium

2. **Frontend-owned UX patch** — add favorite toggle in Svelte and keep shuffle/repeat as client-only flags around existing next/previous behavior.
   - Pros: Faster initial UI work.
   - Cons: Violates backend-driven architecture; cannot reliably persist history on real playback start; queue mode can desync and current IPC lacks enough metadata.
   - Effort: Medium

### Recommendation
Use **Backend-owned playback modes**. The clean history hook is inside `PlaybackService::play_local()` immediately after state is updated with the new `Track` and before/around the `Playing` transition, because that is the one place local playback actually starts. If `PlaybackService` gains access to `LibraryService` (or a narrow history recorder dependency), `next()` and `previous()` inherit history recording automatically whenever they delegate to `play_local()`. For favorites, add a frontend store `toggle(track)` helper and wire the button into `NowPlayingInfo.svelte`, but also add backend IPC for toggle if the product really wants a first-class toggle command; despite the prompt, no Rust `toggle_favorite` command/service exists in the inspected code. For shuffle/repeat, extend `QueueState` and emit a richer queue snapshot instead of only `Track[]`.

### Risks
- The current frontend calls `commands.play()` for both remote URLs and local file paths, but `PlaybackService::play()` still returns `PlatformNotSupported`; playback paths are already inconsistent, which can complicate UX changes around queue/history.
- `get_queue()` and `queue-updated` expose only tracks, not `current_index` or mode flags, so adding shuffle/repeat without a contract change will create duplicated state.
- Favorites are not preloaded globally; a Now Playing heart can render stale state unless the favorites store is loaded at app startup or on first toggle.

### Ready for Proposal
Yes — tell the user the codebase is ready for a proposal, but the proposal should explicitly include: (1) injecting history recording into backend playback start, (2) adding a real queue snapshot contract with shuffle/repeat/current index, and (3) deciding whether favorite toggle becomes a new IPC command or remains a frontend composition over add/remove.
