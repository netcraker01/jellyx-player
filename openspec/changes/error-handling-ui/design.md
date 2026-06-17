# Design: Error Handling UI (Toast/Notification System)

## Technical Approach

Create a centralized notification store (`notifications.ts`) that all feature stores import and push to on errors/successes. Render toasts via `ToastContainer.svelte` + `Toast.svelte` in App.svelte. Replace all `console.error` calls in stores and actions with typed notification pushes.

## Architecture Decisions

### Decision: Notification store pattern

**Choice**: Custom store wrapper around Svelte writable (same pattern as `createSearchStore`, `createFavoritesStore`)
**Alternatives**: Svelte context API, event bus, Tauri-level interceptor
**Rationale**: Follows existing project convention. All stores use writable-based custom stores. Interceptor was rejected (too generic, duplicate risk). Context API is for component trees, not cross-feature communication.

### Decision: Auto-dismiss via setTimeout per notification

**Choice**: Each notification gets its own `setTimeout` that calls `dismiss(id)`. Timer ID stored in a Map inside the store closure.
**Alternatives**: Single interval polling, CSS animation-only (no removal)
**Rationale**: Simple, predictable, no race conditions. Map<id, timerId> allows cancellation on manual dismiss. CSS-only wouldn't clean up the store array.

### Decision: Max 5 visible toasts — enforce on push

**Choice**: When `push()` is called and length >= 5, call `dismiss()` on the oldest before adding.
**Alternatives**: CSS overflow hidden, virtual scrolling
**Rationale**: Enforcing at store level keeps DOM lean and avoids layout thrash. CSS overflow hides but doesn't remove from DOM.

### Decision: Toast positioning — fixed bottom-right above BottomBar

**Choice**: `position: fixed; bottom: calc(var(--bottombar-height) + 1rem); right: 1rem;`
**Alternatives**: Top-right, bottom-center, absolute positioned
**Rationale**: Bottom-right is standard toast position. Must clear BottomBar (72px). Top-right conflicts with ambient blur overlay.

## Data Flow

```
Feature Stores (player, search, favorites, library, actions)
    │
    ├── on error ──→ notifications.push({ type: 'error', title, message })
    ├── on success ─→ notifications.push({ type: 'success', title, message })
    │
    ▼
notifications store (writable<Notification[]>)
    │
    ▼
ToastContainer.svelte (subscribed to $notifications)
    │
    ├── Toast.svelte (each notification)
    │       ├── slide-in animation
    │       ├── auto-dismiss timer
    │       └── close button → notifications.dismiss(id)
    │
    └── Max 5 visible, oldest auto-dismissed
```

## File Changes

| File | Action | Description |
|------|--------|-------------|
| `ui/src/shared/stores/notifications.ts` | Create | Notification store with push/dismiss/clear + auto-dismiss timers |
| `ui/src/shared/components/Toast.svelte` | Create | Individual toast with type-colored border, close btn, animations |
| `ui/src/shared/components/ToastContainer.svelte` | Create | Fixed container rendering $notifications |
| `ui/src/app/App.svelte` | Modify | Import + mount ToastContainer |
| `ui/src/styles/tokens.css` | Modify | Add --color-error, --color-success, --color-warning, --color-info tokens |
| `ui/src/styles/animations.css` | Modify | Add toastSlideIn, toastSlideOut keyframes |
| `ui/src/features/player/stores/player.ts` | Modify | Replace 7 console.error with notifications.push |
| `ui/src/features/search/stores/search.ts` | Modify | Add notification.push on search error |
| `ui/src/features/favorites/stores/favorites.ts` | Modify | Replace 3 console.error + add success on add |
| `ui/src/features/library/stores/library.ts` | Modify | Replace 3 console.error + add success on scan |
| `ui/src/shared/utils/actions.ts` | Modify | Replace 2 console.error with notifications.push |
| `ui/src/i18n/locales/en.json` | Modify | Add `toasts` namespace |
| `ui/src/i18n/locales/es.json` | Modify | Add `toasts` namespace |

## Interfaces / Contracts

```typescript
// Notification type
export type NotificationType = 'error' | 'warning' | 'success' | 'info';

export interface Notification {
  id: string;
  type: NotificationType;
  title: string;
  message: string;
  timestamp: number;
  dismissible: boolean;
}

// Store interface
export interface NotificationStore {
  subscribe: typeof writable<Notification[]>['subscribe'];
  push: (notification: Omit<Notification, 'id' | 'timestamp'>) => void;
  dismiss: (id: string) => void;
  clear: () => void;
}
```

## Testing Strategy

| Layer | What to Test | Approach |
|-------|-------------|----------|
| Unit | NotificationStore push/dismiss/clear/max | vitest with jsdom |
| Unit | Auto-dismiss timer behavior | vi.useFakeTimers |
| Component | Toast renders with correct variant | @testing-library/svelte |
| Integration | ToastContainer renders notifications | @testing-library/svelte |

## Migration / Rollout

No migration required. Additive change — all existing behavior preserved, notifications are added on top.

## Open Questions

None — design is complete and ready for tasks.