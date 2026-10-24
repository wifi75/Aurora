import { useEffect, useState } from 'react';

export type Route = { name: 'live' } | { name: 'pair'; code: string };

/** Due sole pagine: Live e associazione. Il link del QR e' `#/pair/<codice>`. */
export function parseRoute(hash: string): Route {
  const match = /^#\/pair(?:\/(.*))?$/.exec(hash);
  if (match) return { name: 'pair', code: (match[1] ?? '').replace(/\D/g, '') };
  return { name: 'live' };
}

export function useRoute(): Route {
  const [route, setRoute] = useState(() => parseRoute(location.hash));
  useEffect(() => {
    const onChange = () => setRoute(parseRoute(location.hash));
    window.addEventListener('hashchange', onChange);
    return () => window.removeEventListener('hashchange', onChange);
  }, []);
  return route;
}
