import { useSyncExternalStore } from 'react';
import type { PanelId } from './layout';

/** Larghezza oltre la quale la pagina Stazione e' quella a finestre; sotto, a schede una alla volta. */
export const PHONE_QUERY = '(max-width: 40rem)';

function query(): MediaQueryList | null {
  return typeof window !== 'undefined' && typeof window.matchMedia === 'function' ? window.matchMedia(PHONE_QUERY) : null;
}

function subscribe(onChange: () => void): () => void {
  const list = query();
  list?.addEventListener('change', onChange);
  return () => list?.removeEventListener('change', onChange);
}

/** Vero su uno schermo stretto (telefono): cambia con la rotazione o il ridimensionamento. */
export function usePhone(): boolean {
  return useSyncExternalStore(subscribe, () => query()?.matches ?? false, () => false);
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
