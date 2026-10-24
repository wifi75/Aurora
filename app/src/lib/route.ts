import { useEffect, useState } from 'react';

export type Route = { name: 'live' } | { name: 'pair'; code: string } | { name: 'settings'; tab?: string } | { name: 'automation' } | { name: 'stats' } | { name: 'guide' };

/** Pagine: Live, associazione, impostazioni, automazioni, statistiche e guida. Il link del QR e' `#/pair/<codice>`; `#/impostazioni/<scheda>` apre una scheda precisa. */
export function parseRoute(hash: string): Route {
  const match = /^#\/pair(?:\/(.*))?$/.exec(hash);
  if (match) return { name: 'pair', code: (match[1] ?? '').replace(/\D/g, '') };
  const settings = /^#\/impostazioni(?:\/([a-z]+))?$/.exec(hash);
  if (settings) return settings[1] ? { name: 'settings', tab: settings[1] } : { name: 'settings' };
  if (hash === '#/automazioni') return { name: 'automation' };
  if (hash === '#/statistiche') return { name: 'stats' };
  if (hash === '#/guida') return { name: 'guide' };
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
