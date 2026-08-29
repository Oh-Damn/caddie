import { useCallback, useEffect, useState } from 'react';
import { currentRoute, navigate, parseRoute, ROUTE_EVENT, type Route } from './routes';

export function useRoute() {
  const [route, setRoute] = useState<Route>(currentRoute);

  useEffect(() => {
    const sync = () => setRoute(parseRoute(window.location.pathname));
    window.addEventListener('popstate', sync);
    window.addEventListener(ROUTE_EVENT, sync);
    return () => {
      window.removeEventListener('popstate', sync);
      window.removeEventListener(ROUTE_EVENT, sync);
    };
  }, []);

  const go = useCallback((path: Route, replace = false) => {
    navigate(path, replace);
    setRoute(path);
  }, []);

  return { route, navigate: go };
}
