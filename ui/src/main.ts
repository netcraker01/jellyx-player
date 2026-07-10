/**
 * Jellyx Player — Main entry point.
 *
 * Initializes i18n, imports global styles,
 * and mounts the App component with svelte-routing.
 */

import { initI18n } from './i18n';
import { initPlayerEvents } from '@features/player/stores/player';
import {
  initUpdaterEvents,
  startPeriodicCheck,
  loadPrefs,
  check as checkUpdates,
} from '@features/updater/updater.store';
import { mount } from 'svelte';
import App from './app/App.svelte';

// Global styles
import './styles/global.css';

async function bootstrap() {
  try {
    await initI18n();
    mount(App, {
      target: document.getElementById('app')!,
    });

    initPlayerEvents().catch((err) => {
      console.error('[Jellyx] Player event init failed:', err);
    });

    // Updater: subscribe to backend `update-available` events, load prefs,
    // trigger a startup check, and start the 24h periodic re-check.
    initUpdaterEvents().catch((err) => {
      console.error('[Jellyx] Updater event init failed:', err);
    });
    loadPrefs().catch((err) => {
      console.error('[Jellyx] Updater prefs load failed:', err);
    });
    // Small delay so the first check doesn't compete with player/library init.
    setTimeout(() => {
      checkUpdates(false).catch((err) => {
        console.error('[Jellyx] Updater startup check failed:', err);
      });
    }, 5000);
    startPeriodicCheck();
  } catch (err) {
    console.error('[Jellyx] Bootstrap failed:', err);
    const app = document.getElementById('app');
    if (!app) {
      return;
    }

    const container = document.createElement('div');
    container.style.color = '#ef4444';
    container.style.padding = '2rem';
    container.style.fontFamily = 'monospace';

    const title = document.createElement('h2');
    title.textContent = 'Jellyx Player failed to start';

    const details = document.createElement('pre');
    details.textContent = err instanceof Error ? err.stack || err.message : String(err);

    container.append(title, details);
    app.replaceChildren(container);
  }
}

bootstrap();
