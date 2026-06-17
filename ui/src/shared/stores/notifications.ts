/**
 * Notification store — centralized toast notification queue.
 *
 * Provides push/dismiss/clear methods with auto-dismiss timers.
 * All feature stores push errors and successes here.
 * ToastContainer subscribes to render toasts in the UI.
 */
import { writable } from 'svelte/store';

// ── Types ──────────────────────────────────────────────────────────

export type NotificationType = 'error' | 'warning' | 'success' | 'info';

export interface Notification {
  id: string;
  type: NotificationType;
  title: string;
  message: string;
  timestamp: number;
  dismissible: boolean;
}

export interface NotificationStore {
  subscribe: typeof writable<Notification[]>['subscribe'];
  push: (notification: Omit<Notification, 'id' | 'timestamp'>) => void;
  dismiss: (id: string) => void;
  clear: () => void;
}

// ── Constants ──────────────────────────────────────────────────────

const MAX_VISIBLE = 5;
const ERROR_DURATION_MS = 8000;
const DEFAULT_DURATION_MS = 5000;

// ── Auto-dismiss timer registry ───────────────────────────────────

const timers = new Map<string, ReturnType<typeof setTimeout>>();

// ── Store implementation ──────────────────────────────────────────

let nextId = 0;

function createNotificationStore(): NotificationStore {
  const { subscribe, set, update } = writable<Notification[]>([]);

  return {
    subscribe,

    /** Push a notification. Auto-dismisses after a type-based duration. */
    push(notification: Omit<Notification, 'id' | 'timestamp'>) {
      const id = `notif-${++nextId}`;
      const timestamp = Date.now();
      const entry: Notification = { ...notification, id, timestamp };

      update((list) => {
        // Enforce max visible — dismiss oldest
        if (list.length >= MAX_VISIBLE && list.length > 0) {
          const oldest = list[0];
          clearTimer(oldest.id);
          list = list.slice(1);
        }
        return [...list, entry];
      });

      // Auto-dismiss timer
      const duration =
        notification.type === 'error' ? ERROR_DURATION_MS : DEFAULT_DURATION_MS;
      const timer = setTimeout(() => {
        dismiss(id);
        timers.delete(id);
      }, duration);
      timers.set(id, timer);
    },

    /** Dismiss a notification by id. */
    dismiss(id: string) {
      clearTimer(id);
      update((list) => list.filter((n) => n.id !== id));
    },

    /** Clear all notifications and timers. */
    clear() {
      timers.forEach((timer) => clearTimeout(timer));
      timers.clear();
      set([]);
    },
  };
}

function clearTimer(id: string) {
  const timer = timers.get(id);
  if (timer) {
    clearTimeout(timer);
    timers.delete(id);
  }
}

export const notifications = createNotificationStore();