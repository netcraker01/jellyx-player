# Proposal: Error Handling UI (Toast/Notification System)

## Intent

Users get ZERO feedback when Tauri commands fail. Errors from missing yt-dlp, failed searches, unavailable audio devices, and network timeouts are silently logged to console. A toast notification system is needed to surface errors, warnings, and success messages to the user.

## Scope

### In Scope
- Notification store (writable Svelte store) with push/dismiss/clear
- Toast component with 4 variants: error, warning, success, info
- ToastContainer component mounted in App.svelte
- Auto-dismiss with configurable duration (errors 8s, others 5s)
- Wire all existing `console.error` catch blocks to push notifications
- Success toasts for: favorite added, scan completed
- CSS tokens for notification colors + toast animations
- i18n keys for toast messages (en + es)

### Out of Scope
- Tauri-level error interceptor (approach rejected in explore — too generic)
- Persistent notification history
- Notification preferences/settings page
- Toast actions (retry button, undo) — future enhancement

## Capabilities

### New Capabilities
- `toast-notifications`: Toast notification system with store, components, and error wiring

### Modified Capabilities
- None — this is additive; existing feature stores gain notification calls but their spec-level behavior doesn't change

## Approach

Centralized notification store + ToastContainer (Approach 1 from explore). Replace all `console.error` catch blocks with `notifications.push()` calls. ToastContainer renders at fixed bottom-right with vertical stack, slide-in animation, colored left border by type.

## Affected Areas

| Area | Impact | Description |
|------|--------|-------------|
| `ui/src/shared/stores/notifications.ts` | New | Notification store with push/dismiss/clear |
| `ui/src/shared/components/Toast.svelte` | New | Individual toast component |
| `ui/src/shared/components/ToastContainer.svelte` | New | Container that renders notification queue |
| `ui/src/app/App.svelte` | Modified | Mount ToastContainer |
| `ui/src/styles/tokens.css` | Modified | Add notification color tokens |
| `ui/src/styles/animations.css` | Modified | Add toast slide-in/out keyframes |
| `ui/src/features/player/stores/player.ts` | Modified | Replace console.error with notifications.push |
| `ui/src/features/search/stores/search.ts` | Modified | Add toast on search error |
| `ui/src/features/favorites/stores/favorites.ts` | Modified | Replace console.error + add success toasts |
| `ui/src/features/library/stores/library.ts` | Modified | Replace console.error + add success toasts |
| `ui/src/shared/utils/actions.ts` | Modified | Replace console.error with notifications.push |
| `ui/src/i18n/locales/en.json` | Modified | Add toast message keys |
| `ui/src/i18n/locales/es.json` | Modified | Add toast message keys |

## Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| Toast spam from rapid errors | Medium | Max 5 visible toasts, older dismissed |
| i18n mismatch with Rust error strings | Low | Use i18n keys for known errors, fallback to raw message |
| Layout overlap with BottomBar | Low | Toast positioned above BottomBar with z-index |

## Rollback Plan

Remove ToastContainer from App.svelte, revert store imports, delete notifications.ts + Toast components. All features continue working with console.error fallback.

## Dependencies

- Svelte 4 writable store (already available)
- lucide-svelte for close icon (already installed)

## Success Criteria

- [ ] Every Tauri command error produces a visible toast
- [ ] Success toasts appear for favorite added and scan completed
- [ ] Toasts auto-dismiss (8s error, 5s others)
- [ ] Toasts dismissible on click/close button
- [ ] No more than 5 toasts visible at once
- [ ] i18n keys exist for all toast messages (en + es)
- [ ] Vite build passes, vitest passes