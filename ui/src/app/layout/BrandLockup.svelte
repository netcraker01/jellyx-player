<script lang="ts">
  /**
   * BrandLockup — Helix visual identity for sidebar header.
   *
   * Derived from assets/brand/HelixIcon.svelte. Strips the outer app-tile
   * chrome (shadow, rounded rect frame) and keeps only the core
   * mark so it scales cleanly inside a header row.
   */
  import HelixLogo from '@shared/components/HelixLogo.svelte';
  export let size: number = 28;
  export let label = 'Helix';
</script>

<div class="brand-lockup" role="banner" aria-label={label}>
  <HelixLogo size={size} label={label} />
  <span class="brand-wordmark">Helix</span>
</div>

<style>
  .brand-lockup {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0 1.5rem 1rem;
    border-bottom: 1px solid var(--border-color, #1f2937);
  }

  .brand-wordmark {
    font-size: 1.5rem;
    font-weight: 700;
    letter-spacing: -0.02em;
    /* Brand gradient mapped from assets/brand/design-tokens.json.
       Falls back gracefully to the existing accent token. */
    background: linear-gradient(135deg, #00E5FF 0%, #8A5CFF 58%, #D946FF 100%);
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
