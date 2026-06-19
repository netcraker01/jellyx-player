import { readable } from 'svelte/store';
import { hashHistory } from './hashHistory';

export const currentPath = readable(hashHistory.location.pathname, (set) => {
  set(hashHistory.location.pathname);
  return hashHistory.listen(({ location }) => set(location.pathname));
});

export function navigate(to: string) {
  hashHistory.navigate(to);
}
