<script lang="ts">
  import { X } from 'lucide-svelte';
  import type { Notification } from '@shared/stores/notifications';

  export let notification: Notification;

  import { createEventDispatcher } from 'svelte';
  const dispatch = createEventDispatcher<{ dismiss: { id: string } }>();

  function handleClose() {
    dispatch('dismiss', { id: notification.id });
  }

  function handleClick() {
    if (notification.dismissible) {
      dispatch('dismiss', { id: notification.id });
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Enter' || e.key === ' ') {
      e.preventDefault();
      handleClick();
    }
  }

  $: borderClass = `toast-border-${notification.type}`;
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<div
  class="toast {borderClass}"
  role="status"
  tabindex="-1"
  on:click={handleClick}
  on:keydown={handleKeydown}
>
  <div class="toast-content">
    <span class="toast-title">{notification.title}</span>
    <span class="toast-message">{notification.message}</span>
  </div>
  {#if notification.dismissible}
    <button class="toast-close" on:click|stopPropagation={handleClose} aria-label="Close notification">
      <X size={14} />
    </button>
  {/if}
</div>

<style>
  .toast {
    display: flex;
    align-items: flex-start;
    gap: 0.75rem;
    padding: 0.75rem 1rem;
    border-radius: 6px;
    background: var(--bg-elevated, #1f2937);
    border-left: 3px solid var(--border-color, #1f2937);
    color: var(--text-primary, #e0e0e0);
    cursor: pointer;
    animation: toastSlideIn 0.3s ease forwards;
    transition: opacity 0.2s, transform 0.2s;
    min-width: 280px;
    max-width: 380px;
    border: none;
    border-left: 3px solid;
    font-family: inherit;
    text-align: left;
  }

  .toast:hover {
    background: var(--bg-surface, #111827);
  }

  .toast-border-error {
    border-left-color: var(--color-error, #ef4444);
  }

  .toast-border-warning {
    border-left-color: var(--color-warning, #f59e0b);
  }

  .toast-border-success {
    border-left-color: var(--color-success, #22c55e);
  }

  .toast-border-info {
    border-left-color: var(--color-info, #3b82f6);
  }

  .toast-content {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
    min-width: 0;
  }

  .toast-title {
    font-size: 0.85rem;
    font-weight: 600;
    color: var(--text-primary, #e0e0e0);
  }

  .toast-message {
    font-size: 0.8rem;
    color: var(--text-secondary, #9ca3af);
    word-break: break-word;
  }

  .toast-close {
    background: none;
    border: none;
    color: var(--text-secondary, #9ca3af);
    cursor: pointer;
    padding: 0.2rem;
    border-radius: 4px;
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    transition: color 0.15s, background 0.15s;
  }

  .toast-close:hover {
    color: var(--text-primary, #e0e0e0);
    background: rgba(255, 255, 255, 0.05);
  }

  @keyframes toastSlideIn {
    from {
      transform: translateX(100%);
      opacity: 0;
    }
    to {
      transform: translateX(0);
      opacity: 1;
    }
  }
</style>