import { writable } from 'svelte/store';
import { hashHistory } from './hashHistory';

const { subscribe, set } = writable(hashHistory.location.pathname);

// Start listening immediately — never tear down.
// This ensures navigation keeps working even if a component
// error temporarily disrupts Svelte's subscriber tracking.
hashHistory.listen(({ location }) => set(location.pathname));

export const currentPath = { subscribe };

type NavigateOptions = {
  replace?: boolean;
  preserveScroll?: boolean;
};

export function navigate(to: string, options?: NavigateOptions) {
  hashHistory.navigate(to, options);
}
