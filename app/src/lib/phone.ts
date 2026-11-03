import { useSyncExternalStore } from 'react';
import type { PanelId } from './layout';

/** Larghezza oltre la quale la pagina Stazione e' quella a finestre; sotto, a schede una alla volta. */
export const PHONE_QUERY = '(max-width: 40rem)';

/** Sotto i 768 px la pagina Stazione diventa il telecomando compatto (PhoneLive) al posto dei pannelli. */
export const REMOTE_QUERY = '(max-width: 767.98px)';

function query(text: string): MediaQueryList | null {
  return typeof window !== 'undefined' && typeof window.matchMedia === 'function' ? window.matchMedia(text) : null;
}

function useMedia(text: string): boolean {
  const subscribe = (onChange: () => void) => {
    const list = query(text);
    list?.addEventListener('change', onChange);
    return () => list?.removeEventListener('change', onChange);
  };
  return useSyncExternalStore(subscribe, () => query(text)?.matches ?? false, () => false);
}

/** Vero su uno schermo stretto (telefono): cambia con la rotazione o il ridimensionamento. */
export function usePhone(): boolean {
  return useMedia(PHONE_QUERY);
}

/** Vero sotto i 768 px: la pagina Stazione si mostra come telecomando compatto. */
export function useRemotePhone(): boolean {
  return useMedia(REMOTE_QUERY);
}

/** Tablet in orizzontale: fra 768 e 1366 px di larghezza, piu' largo che alto. */
export const TABLET_QUERY = '(min-width: 768px) and (max-width: 1366px) and (orientation: landscape)';
const COARSE_QUERY = '(pointer: coarse)';

/** Telefono, tablet o un altro PC sulla rete: la pagina non e' aperta dal PC dove gira Aurora (ne' dalla sua finestra). */
export const REMOTE_HOST =
  typeof window !== 'undefined' &&
  !('__TAURI_INTERNALS__' in window) &&
  !['localhost', '127.0.0.1', '[::1]'].includes(window.location.hostname);

export type StationView = 'phone' | 'tablet' | 'desktop';

/** La vista chiesta dall'indirizzo (`?vista=telefono` o `?vista=tablet`), usata dalle anteprime dal vivo delle Impostazioni. */
export function forcedView(search: string): StationView | null {
  const v = new URLSearchParams(search).get('vista');
  if (v === 'telefono') return 'phone';
  if (v === 'tablet') return 'tablet';
  return null;
}

/**
 * Quale pagina Stazione mostrare: il telecomando sotto i 768 px; la vista tablet a due colonne fra 768 e 1366 px in orizzontale
 * solo su un dispositivo remoto o a tocco (sul PC normale non cambia niente); altrimenti quella a finestre.
 */
export function useStationView(): StationView {
  const phone = useMedia(REMOTE_QUERY);
  const tablet = useMedia(TABLET_QUERY);
  const coarse = useMedia(COARSE_QUERY);
  const forced = typeof window !== 'undefined' ? forcedView(window.location.search) : null;
  if (forced) return forced;
  if (phone) return 'phone';
  if (tablet && (REMOTE_HOST || coarse)) return 'tablet';
  return 'desktop';
}

/** Le schede della pagina Stazione sul telefono, nell'ordine in cui servono durante un collegamento. */
export const PHONE_TABS: readonly PanelId[] = ['all', 'cq', 'tx', 'station', 'waterfall', 'cluster', 'log'];

const KEY = 'aurora-phone-tab';

type ReadableStorage = Pick<Storage, 'getItem'> | null;
type WritableStorage = Pick<Storage, 'setItem'> | null;

export function readPhoneTab(storage: ReadableStorage): PanelId {
  try {
    const stored = storage?.getItem(KEY);
    return PHONE_TABS.find((id) => id === stored) ?? 'all';
  } catch {
    return 'all';
  }
}

export function savePhoneTab(storage: WritableStorage, id: PanelId): void {
  try {
    storage?.setItem(KEY, id);
  } catch {
    // senza archivio si riparte dalla prima scheda
  }
}
