<script lang="ts">
  import { navigate } from '@app/router/navigation';
  import { Search } from 'lucide-svelte';
  import { t } from '@i18n';
  import HelixLogo from '@shared/components/HelixLogo.svelte';
</script>

<div class="page-home">
  <div class="hero-center">
    <div class="brand-glow" aria-hidden="true"></div>
    <HelixLogo size={96} />

    <h1 class="brand-heading">
      {$t('home.welcome')}
    </h1>
    <p class="tagline">
      {($t('home.tagline') !== 'home.tagline' ? $t('home.tagline') : 'Your sound, your space.')}
    </p>

    <button class="search-btn" on:click={() => navigate('/search')}>
      <Search size={18} />
      {$t('common.search')}
    </button>
  </div>
</div>

<style>
  .page-home {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100%;
    padding: 2rem;
    /* Midnight tonal wash aligned with imagen/helix-theme.css */
    background: radial-gradient(
      ellipse at 50% 30%,
      rgba(109, 92, 255, 0.08) 0%,
      transparent 60%
    ),
    var(--bg-base, #0a0a0f);
  }

  .hero-center {
    position: relative;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 1.25rem;
    text-align: center;
    z-index: 1;
  }

  /* Subtle ambient glow behind the logo — purely decorative */
  .brand-glow {
    position: absolute;
    top: -2rem;
    width: 12rem;
    height: 12rem;
    border-radius: 50%;
    background: radial-gradient(
      circle,
      rgba(0, 229, 255, 0.15) 0%,
      rgba(138, 92, 255, 0.08) 45%,
      transparent 70%
    );
    filter: blur(24px);
    pointer-events: none;
    z-index: -1;
  }

  .brand-heading {
    margin: 0;
    font-size: 2rem;
    font-weight: 700;
    letter-spacing: -0.02em;
    /* Brand gradient mapped from imagen/helix_design_tokens.json */
    background: linear-gradient(135deg, #00E5FF 0%, #8A5CFF 58%, #D946FF 100%);
    -webkit-background-clip: text;
    -webkit-text-fill-color: transparent;
    background-clip: text;
    color: var(--color-accent, #6366f1);
  }

  @supports not (-webkit-background-clip: text) {
    .brand-heading {
      color: var(--color-accent, #6366f1);
      -webkit-text-fill-color: unset;
    }
  }

  .tagline {
    margin: 0;
    font-size: 1rem;
    color: var(--text-secondary, #9ca3af);
  }

  .search-btn {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    margin-top: 0.5rem;
    padding: 0.65rem 1.75rem;
    border: 1px solid var(--color-accent, #6366f1);
    border-radius: 24px;
    background: transparent;
    color: var(--color-accent, #6366f1);
    font-size: 1rem;
    cursor: pointer;
    transition: background 0.2s, color 0.2s, box-shadow 0.2s;
  }

  .search-btn:hover {
    background: var(--color-accent, #6366f1);
    color: #ffffff;
    box-shadow: 0 0 12px rgba(138, 92, 255, 0.35);
  }

  .search-btn:active {
    transform: translateY(1px);
  }
</style>