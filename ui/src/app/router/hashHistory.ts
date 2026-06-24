type LocationShape = {
  pathname: string;
  search: string;
  hash: string;
  state?: unknown;
  key?: string;
};

type Listener = (event: {
  location: LocationShape;
  action: 'POP' | 'PUSH';
  preserveScroll?: boolean;
}) => void;

function normalizePath(path: string | null | undefined): string {
  if (!path || path === '#') return '/';

  const withoutHash = path.startsWith('#') ? path.slice(1) : path;
  return withoutHash.startsWith('/') ? withoutHash : `/${withoutHash}`;
}

function createMemoryLocation(pathname = '/'): LocationShape {
  return {
    pathname,
    search: '',
    hash: '',
    state: undefined,
    key: 'initial',
  };
}

function getWindowLocation(): LocationShape {
  if (typeof window === 'undefined') {
    return createMemoryLocation('/');
  }

  return {
    pathname: normalizePath(window.location.hash),
    search: '',
    hash: window.location.hash,
    state: window.history.state,
    key: (window.history.state as { key?: string } | null)?.key ?? 'initial',
  };
}

function createHashHistory() {
  if (typeof window === 'undefined') {
    let location = createMemoryLocation('/');
    const listeners: Listener[] = [];

    return {
      get location() {
        return location;
      },
      listen(listener: Listener) {
        listeners.push(listener);
        return () => {
          const index = listeners.indexOf(listener);
          if (index >= 0) listeners.splice(index, 1);
        };
      },
      navigate(to: string, { preserveScroll = false } = {}) {
        location = {
          ...createMemoryLocation(normalizePath(to)),
          key: `${Date.now()}`,
        };
        listeners.forEach((listener) => listener({ location, action: 'PUSH', preserveScroll }));
      },
    };
  }

  let location = getWindowLocation();
  const listeners: Listener[] = [];

  const emit = (action: 'POP' | 'PUSH', preserveScroll = false) => {
    location = getWindowLocation();
    listeners.forEach((listener) => listener({ location, action, preserveScroll }));
  };

  return {
    get location() {
      location = getWindowLocation();
      return location;
    },
    listen(listener: Listener) {
      listeners.push(listener);
      const onHashChange = () => emit('POP');
      window.addEventListener('hashchange', onHashChange);

      return () => {
        window.removeEventListener('hashchange', onHashChange);
        const index = listeners.indexOf(listener);
        if (index >= 0) listeners.splice(index, 1);
      };
    },
    navigate(to: string, { replace = false, preserveScroll = false } = {}) {
      const normalized = normalizePath(to);
      const state = { ...(window.history.state ?? {}), key: `${Date.now()}` };
      const url = `${window.location.pathname}#${normalized}`;

      if (replace) {
        window.history.replaceState(state, '', url);
      } else {
        window.history.pushState(state, '', url);
      }

      emit('PUSH', preserveScroll);
    },
  };
}

export const hashHistory = createHashHistory();
