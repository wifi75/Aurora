import { create } from 'zustand';
import type { DecodedMessage } from '../api/schemas';
import { callMatches, callOf } from './filters';
import { browserStorage } from './theme';

/** Le categorie di messaggio che la finestra «CQ e novità» può elencare (le etichette del motore). */
export const CQ_TAGS = [
  'cq', 'new_dxcc', 'new_dxcc_band', 'new_zone', 'new_zone_band', 'new_itu', 'new_itu_band',
  'new_grid', 'new_grid_band', 'new_px', 'new_px_band', 'new_call', 'new_call_band', 'worked_call',
] as const;
export type CqTag = (typeof CQ_TAGS)[number];

/** Cosa elencare in «CQ e novità»: categorie, e due liste di nominativi (con * finale) per includere o escludere stazioni. */
export interface CqPrefs {
  tags: Record<CqTag, boolean>;
  /** Se non vuota, si elencano SOLO queste stazioni (es. JA*, EA8*, K1ABC). */
  only: string[];
  /** Queste stazioni non si elencano mai. */
  hide: string[];
  /** Quante righe al massimo. */
  limit: number;
}

const KEY = 'aurora-cq-prefs';
type ReadableStorage = Pick<Storage, 'getItem'> | null;
type WritableStorage = Pick<Storage, 'setItem'> | null;

export function defaultCqPrefs(): CqPrefs {
  const tags = {} as Record<CqTag, boolean>;
  for (const t of CQ_TAGS) tags[t] = true;
  return { tags, only: [], hide: [], limit: 40 };
}

const strings = (v: unknown): string[] => (Array.isArray(v) ? v.filter((x): x is string => typeof x === 'string' && x.trim() !== '').map((x) => x.trim().toUpperCase()) : []);

export function readCqPrefs(storage: ReadableStorage): CqPrefs {
  const prefs = defaultCqPrefs();
  try {
    const raw = storage?.getItem(KEY);
    if (!raw) return prefs;
    const p = JSON.parse(raw) as Record<string, unknown>;
    if (typeof p.tags === 'object' && p.tags !== null) {
      for (const t of CQ_TAGS) {
        const v = (p.tags as Record<string, unknown>)[t];
        if (typeof v === 'boolean') prefs.tags[t] = v;
      }
    }
    prefs.only = strings(p.only);
    prefs.hide = strings(p.hide);
    if (typeof p.limit === 'number' && p.limit >= 5 && p.limit <= 200) prefs.limit = Math.round(p.limit);
  } catch {
    return defaultCqPrefs();
  }
  return prefs;
}

export function saveCqPrefs(storage: WritableStorage, prefs: CqPrefs): void {
  try {
    storage?.setItem(KEY, JSON.stringify(prefs));
  } catch {
    // senza archivio la scelta non viene ricordata, ma la finestra funziona
  }
}

/** I messaggi da elencare in «CQ e novità» secondo le scelte; ordine invariato. */
export function applyCqPrefs(decodes: DecodedMessage[], prefs: CqPrefs): DecodedMessage[] {
  return decodes
    .filter((d) => {
      if (d.tag === null || !prefs.tags[d.tag as CqTag]) return false;
      if (prefs.only.length === 0 && prefs.hide.length === 0) return true;
      const call = callOf(d.text);
      if (prefs.hide.some((p) => callMatches(call, p))) return false;
      return prefs.only.length === 0 || prefs.only.some((p) => callMatches(call, p));
    })
    .slice(0, prefs.limit);
}

interface CqPrefsState {
  prefs: CqPrefs;
  set(change: Partial<CqPrefs>): void;
  reset(): void;
}

export const useCqPrefs = create<CqPrefsState>((set, get) => {
  const commit = (next: CqPrefs) => {
    saveCqPrefs(browserStorage(), next);
    set({ prefs: next });
  };
  return {
    prefs: readCqPrefs(browserStorage()),
    set: (change) => commit({ ...get().prefs, ...change }),
    reset: () => commit(defaultCqPrefs()),
  };
});
