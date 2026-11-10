import { useEffect, useState } from 'react';

export type Route = { name: 'digi' } | { name: 'fox' } | { name: 'contest' } | { name: 'qmap' } | { name: 'live' } | { name: 'pair'; code: string; alt?: string } | { name: 'settings'; tab?: string } | { name: 'stats' } | { name: 'map' } | { name: 'dxmap' } | { name: 'history' } | { name: 'propagation' } | { name: 'rotator' } | { name: 'sstv' } | { name: 'guide' } | { name: 'news' } | { name: 'ownerMessages' } | { name: 'export' } | { name: 'window'; id: string };

/** Pagine: Live, associazione, impostazioni, automazioni, statistiche, guida, novita' ed esportazione del log (`#/esporta`). Il link del QR e' `#/pair/<codice>`; `#/impostazioni/<scheda>` apre una scheda precisa. */
export function parseRoute(hash: string): Route {
  // link del QR: `#/associa?codice=12345678&nome=aurora.local` (il nome e' l'indirizzo fisso da ricordare)
  const assoc = /^#\/associa(?:\?(.*))?$/.exec(hash);
  if (assoc) {
    const q = new URLSearchParams(assoc[1] ?? '');
    const alt = (q.get('nome') ?? '').replace(/[^a-zA-Z0-9.-]/g, '');
    return alt ? { name: 'pair', code: (q.get('codice') ?? '').replace(/\D/g, ''), alt } : { name: 'pair', code: (q.get('codice') ?? '').replace(/\D/g, '') };
  }
  const match = /^#\/pair(?:\/(.*))?$/.exec(hash);
  if (match) return { name: 'pair', code: (match[1] ?? '').replace(/\D/g, '') };
  const settings = /^#\/impostazioni(?:\/([a-z]+))?$/.exec(hash);
  if (settings) return settings[1] ? { name: 'settings', tab: settings[1] } : { name: 'settings' };
  // le Automazioni stanno nelle Impostazioni, nella loro scheda
  if (hash === '#/automazioni') return { name: 'settings', tab: 'automation' };
  if (hash === '#/statistiche') return { name: 'stats' };
  // la vecchia «Mappa» e' confluita nella Mappa DX: il suo indirizzo porta li'
  if (hash === '#/mappa') return { name: 'dxmap' };
  if (hash === '#/mappa-dx') return { name: 'dxmap' };
  if (hash === '#/storico') return { name: 'history' };
  if (hash === '#/propagazione') return { name: 'propagation' };
  if (hash === '#/rotore') return { name: 'rotator' };
  if (hash === '#/sstv') return { name: 'sstv' };
  if (hash === '#/testo') return { name: 'digi' };
  // `#/guida/<sezione>`: la Guida aperta su una sezione (per esempio dalla scheda SDR delle Impostazioni)
  if (hash === '#/guida' || /^#\/guida\/[a-z0-9-]{1,30}$/.test(hash)) return { name: 'guide' };
  if (hash === '#/novita') return { name: 'news' };
  // pagina dei messaggi agli utenti: non e' in nessun menu, la apre solo l'icona sul desktop dell'autore
  if (hash === '#/messaggi-autore') return { name: 'ownerMessages' };
  if (hash === '#/fox') return { name: 'fox' };
  if (hash === '#/contest') return { name: 'contest' };
  if (hash === '#/qmap') return { name: 'qmap' };
  if (hash === '#/esporta') return { name: 'export' };
  // una pagina da sola in una finestra a parte (secondo schermo): solo il contenuto, senza menu
  const solo = /^#\/finestra\/([a-z0-9]{1,12})$/.exec(hash);
  if (solo) return { name: 'window', id: solo[1] ?? '' };
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
