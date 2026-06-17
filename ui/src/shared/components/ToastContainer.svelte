<script lang="ts">
  import { notifications } from '@shared/stores/notifications';
  import Toast from './Toast.svelte';

  function handleDismiss(e: CustomEvent<{ id: string }>) {
    notifications.dismiss(e.detail.id);
  }
</script>

{#if $notifications.length > 0}
  <div class="toast-container">
    {#each $notifications as notification (notification.id)}
      <Toast {notification} on:dismiss={handleDismiss} />
    {/each}
  </div>
{/if}

<style>
  .toast-container {
    position: fixed;
    bottom: calc(var(--bottombar-height, 72px) + 1rem);
    right: 1rem;
    z-index: 1000;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    pointer-events: none;
  }

  .toast-container > :global(*) {
    pointer-events: auto;
  }
</style>