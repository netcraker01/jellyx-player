<script lang="ts">
  /**
   * BrandLockup — Jellyx visual identity for sidebar header.
   *
   * Derived from assets/brand/jellyx_brand_assets/JellyxIcon.svelte. Strips the outer app-tile
   * chrome (shadow, rounded rect frame) and keeps only the core
   * mark so it scales cleanly inside a header row.
   */
  import JellyxLogo from '@shared/components/JellyxLogo.svelte';
  export let size: number = 36;
  export let label = 'Jellyx';
  export let collapsed = false;
</script>

<div class="brand-lockup" class:collapsed role="banner" aria-label={label}>
  <JellyxLogo size={size} label={label} />
  <span class="brand-wordmark">Jellyx</span>
</div>

<style>
  .brand-lockup {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0 1.25rem 1rem;
    border-bottom: 1px solid var(--border-color, #1f2937);
  }

  .brand-lockup.collapsed {
    justify-content: center;
    padding: 0 0 1rem;
  }

  .brand-lockup.collapsed .brand-wordmark {
    display: none;
  }

  .brand-lockup.collapsed :global(svg) {
    width: 40px;
    height: 40px;
  }

  .brand-wordmark {
    font-size: 1.75rem;
    font-weight: 700;
    letter-spacing: -0.02em;
    /* Brand gradient mapped from assets/brand/design-tokens.json.
       Falls back gracefully to the existing accent token. */
    background: var(--jellyx-gradient-primary, linear-gradient(135deg, #00E5FF 0%, #8A5CFF 58%, #D946FF 100%));
    -webkit-background-clip: text;
    -webkit-text-fill-color: transparent;
    background-clip: text;
    color: var(--color-accent, #6366f1);
  }

  /* Fallback for browsers without background-clip:text support */
  @supports not (-webkit-background-clip: text) {
    .brand-wordmark {
      color: var(--color-accent, #6366f1);
      -webkit-text-fill-color: unset;
    }
  }
</style>
