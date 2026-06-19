/**
 * Helix — Main entry point.
 *
 * Initializes i18n, imports global styles,
 * and mounts the App component with svelte-routing.
 */

import { initI18n } from './i18n';
import { initPlayerEvents } from '@features/player/stores/player';
import App from './app/App.svelte';

// Global styles
import './styles/global.css';

async function bootstrap() {
  try {
    await initI18n();
    new App({
      target: document.getElementById('app')!,
    });

    initPlayerEvents().catch((err) => {
      console.error('[Helix] Player event init failed:', err);
    });
  } catch (err) {
    console.error('[Helix] Bootstrap failed:', err);
    const app = document.getElementById('app');
    if (!app) {
      return;
    }

    const container = document.createElement('div');
    container.style.color = '#ef4444';
    container.style.padding = '2rem';
    container.style.fontFamily = 'monospace';

    const title = document.createElement('h2');
    title.textContent = 'Helix failed to start';

    const details = document.createElement('pre');
    details.textContent = err instanceof Error ? err.stack || err.message : String(err);

    container.append(title, details);
    app.replaceChildren(container);
  }
}

bootstrap();
