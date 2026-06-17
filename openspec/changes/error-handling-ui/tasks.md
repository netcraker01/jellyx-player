# Tasks: Error Handling UI (Toast/Notification System)

## Review Workload Forecast

| Field | Value |
|-------|-------|
| Estimated changed lines | ~250-300 |
| 400-line budget risk | Low |
| Chained PRs recommended | No |
| Suggested split | Single PR |
| Delivery strategy | ask-on-risk |
| Chain strategy | pending |

Decision needed before apply: No
Chained PRs recommended: No
Chain strategy: pending
400-line budget risk: Low

## Phase 1: Foundation

- [x] 1.1 Create `ui/src/shared/stores/notifications.ts` — NotificationStore with push/dismiss/clear, auto-dismiss timers (8s error, 5s others), max 5 cap
- [x] 1.2 Add notification color tokens to `ui/src/styles/tokens.css` — --color-error, --color-success, --color-warning, --color-info
- [x] 1.3 Add toast animation keyframes to `ui/src/styles/animations.css` — toastSlideIn, toastSlideOut
- [x] 1.4 Add `toasts` i18n namespace to `ui/src/i18n/locales/en.json` — favorite_added, scan_completed, track_added_to_queue, yt_dlp_missing
- [x] 1.5 Add `toasts` i18n namespace to `ui/src/i18n/locales/es.json` — same keys with Spanish translations

## Phase 2: UI Components

- [x] 2.1 Create `ui/src/shared/components/Toast.svelte` — single toast with type-colored left border, title, message, close button (X icon from lucide-svelte), slide-in/out animation, click-to-dismiss
- [x] 2.2 Create `ui/src/shared/components/ToastContainer.svelte` — fixed bottom-right container, subscribes to $notifications, renders Toast for each, stacks vertically newest at bottom
- [x] 2.3 Mount ToastContainer in `ui/src/app/App.svelte` — import and add `<ToastContainer />` after BottomBar

## Phase 3: Error Wiring

- [x] 3.1 Wire `ui/src/features/player/stores/player.ts` — replace 7 console.error with notifications.push({ type: 'error', title: 'Playback Error', message })
- [x] 3.2 Wire `ui/src/features/search/stores/search.ts` — add notifications.push on search error, keep searchError writable
- [x] 3.3 Wire `ui/src/features/favorites/stores/favorites.ts` — replace 3 console.error with error pushes, add success push on add using 'Added to favorites'
- [x] 3.4 Wire `ui/src/features/library/stores/library.ts` — replace 3 console.error with error pushes, add success push on scan complete with file count
- [x] 3.5 Wire `ui/src/shared/utils/actions.ts` — replace 2 console.error with notifications.push, add success push on addToQueue using 'Track added to queue'

## Phase 4: Build Verification

- [x] 4.1 Run `vite build` and verify no TypeScript/build errors
- [x] 4.2 Run `vitest run` and verify all existing tests pass