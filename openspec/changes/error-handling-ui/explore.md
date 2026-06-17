## Exploration: Toast/Notification System for Error Handling

### Current State

Errors from Tauri commands are currently handled via:
1. **`console.error()`** in all stores — player.ts, favorites.ts, library.ts, actions.ts (8 catch blocks total)
2. **Local error state** in some stores:
   - `searchError` (writable<string | null>) in search store — displayed inline in ResultsList
   - `scanError` (writable<string | null>) in library store — displayed inline in Library Page
3. **Silent swallowing** — favorites.load(), favorites.add(), favorites.remove() catch and log but show nothing to user
4. **Player errors** — playTrack, pauseTrack, resumeTrack, nextTrack, previousTrack, seekTo, setVolume all catch and console.error only
5. **Library errors** — loadWatchedFolders, loadLocalTracks, removeFolder catch and console.error only

No global notification system exists. Users see ZERO feedback when:
- yt-dlp is missing or search fails
- Audio device unavailable
- Favorite add/remove fails
- Folder scan fails
- Network errors during playback

### Affected Areas

- `ui/src/services/tauri.ts` — IPC abstraction (needs error interception hook)
- `ui/src/services/commands.ts` — 12 command wrappers (all lack error handling)
- `ui/src/features/player/stores/player.ts` — 7 action methods with console.error
- `ui/src/features/search/stores/search.ts` — search method with searchError writable
- `ui/src/features/favorites/stores/favorites.ts` — 3 methods with console.error
- `ui/src/features/library/stores/library.ts` — 4 methods with console.error
- `ui/src/shared/utils/actions.ts` — 2 action methods with console.error
- `ui/src/app/App.svelte` — needs ToastContainer mounted
- `ui/src/styles/tokens.css` — needs notification color tokens
- `ui/src/styles/animations.css` — needs toast animation keyframes
- `ui/src/i18n/locales/en.json` — has errors section already (7 keys)
- `ui/src/i18n/locales/es.json` — has errors section already (7 keys)

### Approaches

1. **Centralized notification store + ToastContainer** — Single `notifications` writable store; all stores push errors to it; ToastContainer renders in App.svelte
   - Pros: Single source of truth, consistent UI, easy to add new error sources, decoupled from feature stores
   - Cons: Requires touching all stores to wire up
   - Effort: Medium

2. **Tauri-level error interceptor** — Wrap `invokeCommand` to catch all errors globally and push to notification store
   - Pros: Catches ALL Tauri errors automatically, zero per-store changes needed
   - Cons: Can't customize per-error context (title, type), loses feature-specific error semantics
   - Effort: Low

3. **Hybrid: Interceptor + per-store customization** — Global interceptor catches Tauri errors and pushes generic notifications; stores can push richer typed notifications
   - Pros: Best of both worlds — no error slips through, rich context when available
   - Cons: Duplicate notifications risk if both interceptor and store push
   - Effort: Medium

### Recommendation

**Approach 1: Centralized notification store + ToastContainer**. The interceptor approach (2/3) risks over-generic error messages and duplicate toasts. A notification store gives each call site control over the title, type (error/warning/success/info), and message. The per-store `console.error` catch blocks become `notifications.push()` calls — simple, explicit, no magic.

### Risks

- Duplicate notifications if multiple error sources fire simultaneously (mitigate with debounce/dedup)
- Toast stacking overflow with rapid errors (mitigate with maxVisible cap)
- i18n: error messages from Rust backend are English strings — toast should use i18n keys when possible, fallback to raw message

### Ready for Proposal
Yes — scope and approach are clear.