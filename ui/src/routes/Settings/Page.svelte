<script lang="ts">
  import { onMount } from 'svelte';
  import { t, locale, switchLocale } from '@i18n';
  import { getVersion, getSourceSettings, setSourceEnabled } from '@services/commands';
  import type { SourceSetting } from '@services/commands';
  import { Library, Info, Languages, Plug } from 'lucide-svelte';

  let version = '';
  let versionError: string | null = null;
  let sourceSettings: SourceSetting[] = [];

  onMount(() => {
    getVersion()
      .then((v) => {
        version = v;
        versionError = null;
      })
      .catch(() => {
        versionError = $t('common.error');
      });

    loadSourceSettings();
  });

  async function loadSourceSettings() {
    try {
      sourceSettings = await getSourceSettings();
    } catch {
      sourceSettings = [];
    }
  }

  async function handleToggle(source: string, enabled: boolean) {
    // Optimistic update
    const previousSettings = [...sourceSettings];
    sourceSettings = sourceSettings.map((s) =>
      s.source === source ? { ...s, enabled: !enabled } : s
    );
    try {
      await setSourceEnabled(source, !enabled);
    } catch {
      // Rollback on error
      sourceSettings = previousSettings;
    }
  }

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
      <Plug size={20} />
      <h2>{$t('settings.sources')}</h2>
    </div>
    <p class="section-desc">{$t('settings.sources_desc')}</p>
    {#each sourceSettings as setting (setting.source)}
      <div class="setting-row">
        <span class="setting-label">{setting.label}</span>
        <label class="toggle">
          <input
            type="checkbox"
            checked={setting.enabled}
            on:change={() => handleToggle(setting.source, setting.enabled)}
          />
          <span class="toggle-slider"></span>
        </label>
      </div>
    {/each}
  </section>

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

  .section-desc {
    color: var(--text-secondary, #9ca3af);
    font-size: 0.85rem;
    margin: 0 0 1rem 0;
  }

  .setting-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 1rem;
    padding: 0.5rem 0;
  }

  .setting-row label {
    color: var(--text-secondary, #9ca3af);
    font-size: 0.9rem;
  }

  .setting-label {
    color: var(--text-primary, #e0e0e0);
    font-size: 0.95rem;
    font-weight: 500;
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

  /* Toggle switch */
  .toggle {
    position: relative;
    display: inline-block;
    width: 44px;
    height: 24px;
    cursor: pointer;
  }

  .toggle input {
    opacity: 0;
    width: 0;
    height: 0;
  }

  .toggle-slider {
    position: absolute;
    inset: 0;
    background: var(--bg-elevated, #1f2937);
    border: 1px solid var(--border-color, #1f2937);
    border-radius: 12px;
    transition: background 0.2s, border-color 0.2s;
  }

  .toggle-slider::before {
    content: '';
    position: absolute;
    width: 18px;
    height: 18px;
    left: 2px;
    bottom: 2px;
    background: var(--text-secondary, #9ca3af);
    border-radius: 50%;
    transition: transform 0.2s, background 0.2s;
  }

  .toggle input:checked + .toggle-slider {
    background: var(--color-accent, #6366f1);
    border-color: var(--color-accent, #6366f1);
  }

  .toggle input:checked + .toggle-slider::before {
    transform: translateX(20px);
    background: white;
  }
</style>