import type { Route } from './route';

/**
 * L'ultima pagina aperta e l'ultima scheda delle impostazioni si ricordano nelle preferenze (chiavi `aurora-...`,
 * che si salvano anche sul server): al prossimo avvio Aurora riparte da dove l'hai lasciata.
 */

const PAGE_KEY = 'aurora-last-page';
const TAB_KEY = 'aurora-settings-tab';
// tutte le pagine dei modi (PSK·RTTY·CW, SSTV, Satelliti, Fox, Contest...) e quelle comuni: riaprendo Aurora si torna li'
const RESTORABLE = new Set(['#/statistiche', '#/mappa-dx', '#/storico', '#/propagazione', '#/impostazioni', '#/guida', '#/novita', '#/testo', '#/sstv', '#/wefax', '#/satelliti', '#/fox', '#/contest', '#/qmap', '#/rotore', '#/esporta']);

/** Le pagine che accendono il motore (ascolto dei modi testuali, SSTV): un client remoto non ci riparte da solo. */
export const ENGINE_PAGES = new Set(['#/testo', '#/sstv', '#/wefax']);

type Store = Pick<Storage, 'getItem' | 'setItem'> | null;

function read(storage: Store, key: string): string | null {
  try {
    return storage?.getItem(key) ?? null;
  } catch {
    return null;
  }
}

function write(storage: Store, key: string, value: string): void {
  try {
    storage?.setItem(key, value);
  } catch {
    // senza archivio del browser non si ricorda nulla, ma l'interfaccia funziona lo stesso
  }
}

/**
 * All'avvio, senza un indirizzo preciso (`#/pair/...`, `#/impostazioni/rig`), si torna all'ultima pagina; la Stazione e' quella di base.
 * Un client remoto (telefono, portatile in rete) non riapre le pagine che accendono il motore: parte dalla Stazione.
 */
export function lastPageToRestore(storage: Store, hash: string, remote = false): string | null {
  if (hash !== '' && hash !== '#' && hash !== '#/') return null;
  const saved = read(storage, PAGE_KEY);
  if (saved === null || !RESTORABLE.has(saved)) return null;
  return remote && ENGINE_PAGES.has(saved) ? null : saved;
}

const HASH_OF: Record<string, string> = {
  live: '#/',
  stats: '#/statistiche',
  map: '#/mappa-dx',
  dxmap: '#/mappa-dx',
  digi: '#/testo',
  sstv: '#/sstv',
  wefax: '#/wefax',
  sat: '#/satelliti',
  fox: '#/fox',
  contest: '#/contest',
  qmap: '#/qmap',
  rotator: '#/rotore',
  export: '#/esporta',
  history: '#/storico',
  propagation: '#/propagazione',
  settings: '#/impostazioni',
  guide: '#/guida',
  news: '#/novita',
};

/** Da chiamare a ogni cambio di pagina. L'associazione non si ricorda: e' un passaggio, non una pagina. */
export function rememberRoute(storage: Store, route: Route): void {
  const hash = HASH_OF[route.name];
  if (hash === undefined) return;
  write(storage, PAGE_KEY, hash);
  if (route.name === 'settings' && route.tab) write(storage, TAB_KEY, route.tab);
}

export function rememberTab(storage: Store, tab: string): void {
  write(storage, TAB_KEY, tab);
}

export function rememberedTab(storage: Store): string | undefined {
  return read(storage, TAB_KEY) ?? undefined;
}
