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
import { checkWelcome } from '@features/welcome/welcomeStore';
import { mount } from 'svelte';
import App from './app/App.svelte';
import { getMigratedItem } from '@shared/utils/storage';

// Global styles
import './styles/global.css';

/** Restore the persisted window decorations preference on Linux.
 *  Tauri always creates the window decorated; we must strip the title bar
 *  here if the user previously opted to hide it, so the state survives restart. */
function restoreDecorations(): void {
  if (typeof navigator === 'undefined' || !/Linux/.test(navigator.userAgent)) return;
  if (getMigratedItem('hide-title-bar') !== 'true') return;
  import('@tauri-apps/api/window')
    .then(({ getCurrentWindow }) => getCurrentWindow().setDecorations(false))
    .catch(() => undefined);
}

async function bootstrap() {
  try {
    await initI18n();
    mount(App, {
      target: document.getElementById('app')!,
    });

    restoreDecorations();

    // Check whether the welcome modal should show (once per version).
    checkWelcome().catch((err) => {
      console.error('[Jellyx] Welcome check failed:', err);
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
