<script lang="ts">
  import { onMount } from 'svelte';
  import { t, locale, switchLocale } from '@i18n';
  import { getVersion } from '@services/commands';
  import { Library, Info, Languages } from 'lucide-svelte';

  let version = '';
  let versionError: string | null = null;

  onMount(() => {
    getVersion()
      .then((v) => {
        version = v;
        versionError = null;
      })
      .catch(() => {
        versionError = $t('common.error');
      });
  });

  function handleLocaleChange(e: Event) {
    const select = e.target as HTMLSelectElement;
    switchLocale(select.value).catch((err) => {
      console.error('Failed to switch locale:', err);
    });
  }

  const SUPPORTED_LOCALES = [
    { code: 'en', label: 'English' },
    { code: 'es', label: 'Español' },
  ];

  $: currentLocale = $locale;
</script>

<div class="page-settings">
  <h1>{$t('settings.title')}</h1>

  <section class="settings-section">
    <div class="section-header">
      <Languages size={20} />
      <h2>{$t('settings.language')}</h2>
    </div>
    <div class="setting-row">
      <label for="locale-select">{$t('settings.language')}</label>
      <select id="locale-select" value={currentLocale} on:change={handleLocaleChange}>
        {#each SUPPORTED_LOCALES as loc}
          <option value={loc.code}>{loc.label}</option>
        {/each}
      </select>
    </div>
  </section>

  <section class="settings-section">
    <div class="section-header">
      <Library size={20} />
      <h2>{$t('settings.about')}</h2>
    </div>
    <div class="setting-row">
      <span class="setting-label">{$t('settings.version')}</span>
      {#if versionError}
        <span class="setting-value error">{versionError}</span>
      {:else if version}
        <span class="setting-value">{version}</span>
      {:else}
        <span class="setting-value muted">{$t('common.loading')}</span>
      {/if}
    </div>
  </section>

  <section class="settings-section">
    <div class="section-header">
      <Info size={20} />
      <h2>{$t('app.title')}</h2>
    </div>
    <p class="tagline">{$t('app.tagline')}</p>
  </section>
</div>

<style>
  .page-settings {
    padding: 1rem;
    color: var(--text-primary, #e0e0e0);
  }

  h1 {
    font-size: 1.5rem;
    margin-bottom: 1.5rem;
  }

  .settings-section {
    margin-bottom: 2rem;
    padding: 1rem;
    background: var(--bg-surface, #111827);
    border: 1px solid var(--border-color, #1f2937);
    border-radius: 8px;
  }

  .section-header {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    margin-bottom: 1rem;
    color: var(--text-primary, #e0e0e0);
  }

  .section-header h2 {
    font-size: 1.1rem;
    margin: 0;
    font-weight: 600;
  }

  .setting-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 1rem;
  }

  .setting-row label {
    color: var(--text-secondary, #9ca3af);
    font-size: 0.9rem;
  }

  .setting-label {
    color: var(--text-secondary, #9ca3af);
    font-size: 0.9rem;
  }

  .setting-value {
    font-size: 0.9rem;
    font-variant-numeric: tabular-nums;
  }

  .setting-value.error {
    color: var(--color-error, #ef4444);
  }

  .setting-value.muted {
    color: var(--text-secondary, #9ca3af);
  }

  select {
    background: var(--bg-elevated, #1f2937);
    color: var(--text-primary, #e0e0e0);
    border: 1px solid var(--border-color, #1f2937);
    border-radius: 6px;
    padding: 0.4rem 0.6rem;
    font-size: 0.9rem;
    cursor: pointer;
  }

  select:focus {
    outline: 2px solid var(--color-accent, #6366f1);
    outline-offset: 1px;
  }

  .tagline {
    color: var(--text-secondary, #9ca3af);
    font-size: 0.9rem;
    margin: 0;
  }
</style>
