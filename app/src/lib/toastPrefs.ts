import { create } from 'zustand';
import { browserStorage } from './theme';

export type ToastKindKey = 'ok' | 'info' | 'error';
export type ToastPosition = 'top' | 'topRight' | 'bottom';
export type ToastSize = 'small' | 'normal' | 'large';

export const TOAST_POSITIONS: readonly ToastPosition[] = ['top', 'topRight', 'bottom'];
export const TOAST_SIZES: readonly ToastSize[] = ['small', 'normal', 'large'];
export const TOAST_KINDS: readonly ToastKindKey[] = ['ok', 'info', 'error'];

/** Come si presentano i messaggi toast (le notifiche a scomparsa in alto): quali, quanto durano, dove e quanto grandi. */
export interface ToastPrefs {
  /** Quali tipi mostrare; gli errori restano sempre visibili. */
  show: Record<ToastKindKey, boolean>;
  /** Secondi di permanenza per tipo. */
  seconds: Record<ToastKindKey, number>;
  position: ToastPosition;
  size: ToastSize;
}

const KEY = 'aurora-toast-prefs';

type ReadableStorage = Pick<Storage, 'getItem'> | null;
type WritableStorage = Pick<Storage, 'setItem'> | null;

export function defaultToastPrefs(): ToastPrefs {
  return { show: { ok: true, info: true, error: true }, seconds: { ok: 4.5, info: 5.5, error: 8 }, position: 'top', size: 'normal' };
}

const clampSeconds = (v: unknown, fallback: number) => (typeof v === 'number' && Number.isFinite(v) ? Math.min(30, Math.max(1, v)) : fallback);

export function readToastPrefs(storage: ReadableStorage): ToastPrefs {
  const prefs = defaultToastPrefs();
  try {
    const raw = storage?.getItem(KEY);
    if (!raw) return prefs;
    const p = JSON.parse(raw) as Record<string, unknown>;
    const show = (p.show ?? {}) as Record<string, unknown>;
    const seconds = (p.seconds ?? {}) as Record<string, unknown>;
    for (const k of TOAST_KINDS) {
      if (typeof show[k] === 'boolean') prefs.show[k] = show[k];
      prefs.seconds[k] = clampSeconds(seconds[k], prefs.seconds[k]);
    }
    // gli errori si vedono sempre
    prefs.show.error = true;
    if (typeof p.position === 'string' && (TOAST_POSITIONS as readonly string[]).includes(p.position)) prefs.position = p.position as ToastPosition;
    if (typeof p.size === 'string' && (TOAST_SIZES as readonly string[]).includes(p.size)) prefs.size = p.size as ToastSize;
  } catch {
    return defaultToastPrefs();
  }
  return prefs;
}

export function saveToastPrefs(storage: WritableStorage, prefs: ToastPrefs): void {
  try {
    storage?.setItem(KEY, JSON.stringify(prefs));
  } catch {
    // senza archivio le scelte non vengono ricordate, ma i messaggi funzionano
  }
}

interface ToastPrefsState {
  prefs: ToastPrefs;
  set(change: Partial<ToastPrefs>): void;
  reset(): void;
}

export const useToastPrefs = create<ToastPrefsState>((set, get) => {
  const commit = (next: ToastPrefs) => {
    next.show.error = true;
    saveToastPrefs(browserStorage(), next);
    set({ prefs: next });
  };
  return {
    prefs: readToastPrefs(browserStorage()),
    set: (change) => commit({ ...get().prefs, ...change }),
    reset: () => commit(defaultToastPrefs()),
  };
});
