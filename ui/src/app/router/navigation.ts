import { writable } from 'svelte/store';
import { hashHistory } from './hashHistory';

const { subscribe, set } = writable(hashHistory.location.pathname);

// Start listening immediately — never tear down.
// This ensures navigation keeps working even if a component
// error temporarily disrupts Svelte's subscriber tracking.
hashHistory.listen(({ location }) => set(location.pathname));

export const currentPath = { subscribe };

export function navigate(to: string) {
  hashHistory.navigate(to);
}
