export const ROUTES = {
  home: '/',
  connect: '/connect',
  apps: '/apps',
  clipboard: '/clipboard',
  ai: '/ai',
  settings: '/settings',
} as const;

export type Route = (typeof ROUTES)[keyof typeof ROUTES];

export const ROUTE_EVENT = 'caddie:route';

export function parseRoute(pathname: string): Route {
  if (pathname === ROUTES.connect || pathname.startsWith('/connect/')) {
    return ROUTES.connect;
  }
  if (pathname === ROUTES.apps || pathname.startsWith('/apps/')) {
    return ROUTES.apps;
  }
  if (pathname === ROUTES.clipboard || pathname.startsWith('/clipboard/')) {
    return ROUTES.clipboard;
  }
  if (pathname === ROUTES.ai || pathname.startsWith('/ai/')) {
    return ROUTES.ai;
  }
  if (pathname === ROUTES.settings || pathname.startsWith('/settings/')) {
    return ROUTES.settings;
  }
  return ROUTES.home;
}

export function currentRoute(): Route {
  return parseRoute(window.location.pathname);
}

export function navigate(path: Route, replace = false): void {
  const url = new URL(window.location.href);
  if (url.pathname === path) {
    return;
  }
  url.pathname = path;
  if (replace) {
    history.replaceState(null, '', url);
  } else {
    history.pushState(null, '', url);
  }
  window.dispatchEvent(new Event(ROUTE_EVENT));
}
