<script lang="ts">
  /**
   * WelcomeModal — shown once per version on first launch after install/update.
   *
   * Displays the brand logo, current version, a short open-source message,
   * telemetry opt-in toggle, and a button linking to GitHub to star the project.
   * State is driven by `welcomeStore`.
   */
  import { onMount } from 'svelte';
  import { welcomeStore, dismissWelcome } from './welcomeStore';
  import { t } from '@i18n';
  import { X, Star, ExternalLink, Heart } from 'lucide-svelte';
  import { JELLYX_REPO_URL, JELLYX_STARGAZERS_URL } from '@shared/constants';
  import { getTelemetrySettings, setTelemetryEnabled, openExternalUrl } from '@services/commands';
  import logoWide from '@shared/assets/jellyx/logo-wide.svg';

  $: open = $welcomeStore.modalOpen;
  $: version = $welcomeStore.version;

  let telemetryEnabled = true;

  onMount(() => {
    getTelemetrySettings()
      .then((s) => { telemetryEnabled = s.enabled; })
      .catch(() => {});
  });

  function handleKeydown(e: KeyboardEvent): void {
    if (e.key === 'Escape') dismissWelcome();
  }

  function openStarPage(): void {
    openExternalUrl(JELLYX_REPO_URL).catch(() => {
      window.open(JELLYX_REPO_URL, '_blank');
    });
  }

  async function handleTelemetryToggle(): Promise<void> {
    const next = !telemetryEnabled;
    telemetryEnabled = next;
    try {
      await setTelemetryEnabled(next);
    } catch {
      telemetryEnabled = !next;
    }
  }

  function handleGetStarted(): void {
    dismissWelcome();
  }
</script>

{#if open}
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    class="dialog-overlay"
    on:click={dismissWelcome}
    on:keydown={handleKeydown}
    role="dialog"
    aria-modal="true"
    aria-labelledby="welcome-modal-title"
    tabindex="-1"
  >
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div class="dialog" on:click|stopPropagation on:keydown={handleKeydown}>
      <header class="dialog-header">
        <button class="close-btn" on:click={dismissWelcome} title={$t('common.close')} type="button" aria-label={$t('common.close')}>
          <X size={16} />
        </button>
      </header>

      <div class="brand-section">
        <img src={logoWide} alt="Jellyx" class="brand-logo" />
      </div>

      <p class="version-line">{$t('welcome.version')}: <span class="version-value">v{version}</span></p>

      <div class="message-section">
        <p class="opensource-message">
          {$t('welcome.opensource_message')}
        </p>
        <p class="community-message">
          {$t('welcome.community_message')}
        </p>
      </div>

      <div class="telemetry-section">
        <label class="telemetry-toggle" for="welcome-telemetry">
          <span class="telemetry-label">
            <input
              type="checkbox"
              id="welcome-telemetry"
              checked={telemetryEnabled}
              on:change={handleTelemetryToggle}
            />
            {$t('welcome.telemetry_label')}
          </span>
        </label>
        <p class="telemetry-desc">
          {$t('welcome.telemetry_desc')}
          <a
            href="https://github.com/netcraker01/jellyx-player/blob/main/docs/operations.md"
            target="_blank"
            rel="noopener noreferrer"
            class="telemetry-link"
          >
            {$t('welcome.telemetry_policy')}
            <ExternalLink size={10} />
          </a>
        </p>
      </div>

      <div class="dialog-actions">
        <button class="btn-primary" on:click={openStarPage} type="button">
          <Star size={16} />
          {$t('welcome.star_github')}
        </button>
        <button class="btn-secondary" on:click={handleGetStarted} type="button">
          {$t('welcome.get_started')}
        </button>
      </div>

      <footer class="dialog-footer">
        <a href={JELLYX_REPO_URL} target="_blank" rel="noopener noreferrer" class="repo-link">
          <ExternalLink size={12} />
          {JELLYX_REPO_URL.replace('https://', '')}
        </a>
        <span class="made-with">
          {$t('welcome.made_with')} <Heart size={12} class="heart" />
        </span>
      </footer>
    </div>
  </div>
{/if}

<style>
  .dialog-overlay {
    position: fixed;
    inset: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    background: rgba(0, 0, 0, 0.6);
    z-index: 200;
  }

  .dialog {
    display: flex;
    flex-direction: column;
    gap: 1.25rem;
    padding: 2rem 2rem 1.5rem;
    background: var(--bg-surface, #111827);
    border-radius: 12px;
    border: 1px solid var(--border-color, #1f2937);
    width: min(90vw, 520px);
    max-height: 85vh;
    overflow-y: auto;
  }

  .dialog-header {
    display: flex;
    justify-content: flex-end;
  }

  .close-btn {
    background: none;
    border: none;
    color: var(--text-secondary, #9ca3af);
    cursor: pointer;
    padding: 0.25rem;
    border-radius: 6px;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: color 0.2s, background 0.2s;
  }

  .close-btn:hover {
    color: var(--text-primary, #e0e0e0);
    background: var(--bg-hover, #374151);
  }

  .brand-section {
    display: flex;
    justify-content: center;
    padding: 0.25rem 0;
  }

  .brand-logo {
    width: min(100%, 320px);
    height: auto;
  }

  .version-line {
    margin: 0;
    font-size: 0.95rem;
    color: var(--text-secondary, #9ca3af);
    text-align: center;
  }

  .version-value {
    color: var(--color-accent, #6366f1);
    font-weight: 600;
  }

  .message-section {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    padding: 0.25rem 0;
  }

  .opensource-message {
    margin: 0;
    font-size: 0.9rem;
    line-height: 1.5;
    color: var(--text-secondary, #9ca3af);
    text-align: center;
  }

  .community-message {
    margin: 0;
    font-size: 0.85rem;
    line-height: 1.4;
    color: var(--text-secondary, #9ca3af);
    text-align: center;
    opacity: 0.85;
  }

  .telemetry-section {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    padding: 0.75rem 1rem;
    background: var(--bg-elevated, #1f2937);
    border-radius: 8px;
  }

  .telemetry-toggle {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    cursor: pointer;
  }

  .telemetry-label {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    font-size: 0.9rem;
    color: var(--text-primary, #e0e0e0);
    cursor: pointer;
  }

  .telemetry-label input[type="checkbox"] {
    width: 16px;
    height: 16px;
    accent-color: var(--color-accent, #6366f1);
    cursor: pointer;
  }

  .telemetry-desc {
    margin: 0;
    font-size: 0.8rem;
    line-height: 1.4;
    color: var(--text-secondary, #9ca3af);
  }

  .telemetry-link {
    color: var(--color-accent, #6366f1);
    text-decoration: none;
    display: inline-flex;
    align-items: center;
    gap: 0.15rem;
  }

  .telemetry-link:hover {
    text-decoration: underline;
  }

  .dialog-actions {
    display: flex;
    justify-content: center;
    gap: 0.75rem;
    flex-wrap: wrap;
    padding-top: 0.25rem;
  }

  .btn-primary {
    display: inline-flex;
    align-items: center;
    gap: 0.4rem;
    padding: 0.6rem 1.25rem;
    border: none;
    border-radius: 8px;
    background: var(--color-accent, #6366f1);
    color: white;
    cursor: pointer;
    font-size: 0.95rem;
    font-family: inherit;
    transition: background 0.2s;
  }

  .btn-primary:hover {
    background: var(--color-accent-hover, #4f52d9);
  }

  .btn-secondary {
    display: inline-flex;
    align-items: center;
    gap: 0.4rem;
    padding: 0.6rem 1.25rem;
    border: 1px solid var(--border-color, #1f2937);
    border-radius: 8px;
    background: transparent;
    color: var(--text-secondary, #9ca3af);
    cursor: pointer;
    font-size: 0.95rem;
    font-family: inherit;
    transition: color 0.2s, border-color 0.2s;
  }

  .btn-secondary:hover {
    color: var(--text-primary, #e0e0e0);
    border-color: var(--text-secondary, #9ca3af);
  }

  .dialog-footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.5rem;
    padding-top: 0.5rem;
    border-top: 1px solid var(--border-color, #1f2937);
  }

  .repo-link {
    display: inline-flex;
    align-items: center;
    gap: 0.25rem;
    font-size: 0.75rem;
    color: var(--text-secondary, #9ca3af);
    text-decoration: none;
    transition: color 0.2s;
  }

  .repo-link:hover {
    color: var(--color-accent, #6366f1);
  }

  .made-with {
    display: inline-flex;
    align-items: center;
    gap: 0.25rem;
    font-size: 0.75rem;
    color: var(--text-secondary, #9ca3af);
  }

  .made-with :global(.heart) {
    color: #ef4444;
  }
</style>
