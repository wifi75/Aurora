import type { Route } from './route';

/**
 * L'ultima pagina aperta e l'ultima scheda delle impostazioni si ricordano nelle preferenze (chiavi `aurora-...`,
 * che si salvano anche sul server): al prossimo avvio Aurora riparte da dove l'hai lasciata.
 */

const PAGE_KEY = 'aurora-last-page';
const TAB_KEY = 'aurora-settings-tab';
const RESTORABLE = new Set(['#/automazioni', '#/statistiche', '#/mappa', '#/storico', '#/propagazione', '#/impostazioni', '#/guida']);

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

/** All'avvio, senza un indirizzo preciso (`#/pair/...`, `#/impostazioni/rig`), si torna all'ultima pagina; la Stazione e' quella di base. */
export function lastPageToRestore(storage: Store, hash: string): string | null {
  if (hash !== '' && hash !== '#' && hash !== '#/') return null;
  const saved = read(storage, PAGE_KEY);
  return saved !== null && RESTORABLE.has(saved) ? saved : null;
}

const HASH_OF: Record<string, string> = {
  live: '#/',
  automation: '#/automazioni',
  stats: '#/statistiche',
  map: '#/mappa',
  history: '#/storico',
  propagation: '#/propagazione',
  settings: '#/impostazioni',
  guide: '#/guida',
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
