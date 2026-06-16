/**
 * Helix — Main entry point.
 *
 * Initializes i18n, imports global styles,
 * and mounts the App component with svelte-routing.
 */

import { initI18n } from './i18n';
import App from './app/App.svelte';

// Global styles
import './styles/global.css';

async function bootstrap() {
  await initI18n();

  const app = new App({
    target: document.getElementById('app')!,
  });
}

bootstrap();