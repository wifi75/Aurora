/**
 * Le frequenze WEFAX dell'utente (Impostazioni › Modi › WEFAX › Frequenze): l'elenco di fabbrica di `wefaxStations.ts` resta com'e',
 * qui si tengono solo le differenze (voci di fabbrica nascoste o cambiate, voci aggiunte). Si salvano nella chiave `aurora-wefax-freq`
 * del browser, che `syncSettings.ts` copia sul server come le altre preferenze. «Ripristina quelle di fabbrica» toglie la chiave.
 * La portante resta 1,9 kHz sotto la frequenza assegnata (WEFAX_CARRIER_OFFSET_HZ).
 */
import { useSyncExternalStore } from 'react';
import { browserStorage } from './theme';
import {
  WEFAX_CARRIER_OFFSET_HZ,
  WEFAX_DEFAULT_IOC,
  WEFAX_DEFAULT_LPM,
  presetAt,
  wefaxGroups,
  wefaxPresets,
  type WefaxGroup,
  type WefaxPreset,
} from './wefaxStations';

export const WEFAX_FREQ_KEY = 'aurora-wefax-freq';
const VERSION = 1;
/** Evento della stessa finestra quando l'elenco cambia (fra finestre diverse arriva `storage`). */
export const WEFAX_FREQ_EVENT = 'aurora-wefax-freq';

export const KHZ_MIN = 100;
export const KHZ_MAX = 30_000;
export const CALL_MAX = 24;
export const PLACE_MAX = 40;
export const BAND_MAX = 16;
const ADDED_MAX = 500;

/** Una frequenza come la scrive l'utente. `band` vuota: la banda si ricava dalla frequenza. */
export interface WefaxEntry {
  call: string;
  place: string;
  khz: number;
  band?: string;
}

export interface WefaxFreqState {
  /** Voci di fabbrica nascoste (id del preset). */
  hidden: string[];
  /** Voci di fabbrica cambiate, per id. */
  edited: Record<string, WefaxEntry>;
  /** Voci aggiunte dall'utente. */
  added: Array<WefaxEntry & { id: string }>;
}

export const EMPTY_STATE: WefaxFreqState = {
  hidden: [],
  edited: {},
  added: [],
};

type Store = Pick<Storage, 'getItem' | 'setItem' | 'removeItem'>;

const str = (v: unknown, max: number): string | null => (typeof v === 'string' && v.trim().length <= max ? v.trim() : null);

/** Una voce letta dall'archivio: null se non e' valida (si scarta senza toccare le altre). */
function cleanEntry(raw: unknown): WefaxEntry | null {
  if (!raw || typeof raw !== 'object') return null;
  const r = raw as Record<string, unknown>;
  const call = str(r.call, CALL_MAX);
  const place = str(r.place ?? '', PLACE_MAX);
  const band = str(r.band ?? '', BAND_MAX);
  const khz = typeof r.khz === 'number' ? r.khz : NaN;
  if (!call || place === null || band === null || !Number.isFinite(khz) || khz < KHZ_MIN || khz > KHZ_MAX) return null;
  return band ? { call, place, khz, band } : { call, place, khz };
}

/** Lo stato salvato, a prova di JSON rotto, versione diversa o voci malformate: nel dubbio l'elenco di fabbrica. */
export function parseWefaxFreq(text: string | null): WefaxFreqState {
  if (!text) return EMPTY_STATE;
  let raw: unknown;
  try {
    raw = JSON.parse(text);
  } catch {
    return EMPTY_STATE;
  }
  if (!raw || typeof raw !== 'object' || (raw as { v?: unknown }).v !== VERSION) return EMPTY_STATE;
  const r = raw as Record<string, unknown>;
  const factory = new Set(wefaxPresets().map((p) => p.id));
  const hidden = Array.isArray(r.hidden) ? [...new Set(r.hidden.filter((x): x is string => typeof x === 'string' && factory.has(x)))] : [];
  const edited: Record<string, WefaxEntry> = {};
  if (r.edited && typeof r.edited === 'object' && !Array.isArray(r.edited)) {
    for (const [id, v] of Object.entries(r.edited as Record<string, unknown>)) {
      const e = factory.has(id) ? cleanEntry(v) : null;
      if (e) edited[id] = e;
    }
  }
  const added: WefaxFreqState['added'] = [];
  const ids = new Set<string>();
  if (Array.isArray(r.added)) {
    for (const v of r.added.slice(0, ADDED_MAX)) {
      const id = v && typeof v === 'object' ? (v as { id?: unknown }).id : undefined;
      const e = cleanEntry(v);
      if (e && typeof id === 'string' && id.startsWith('u:') && !ids.has(id)) {
        ids.add(id);
        added.push({ id, ...e });
      }
    }
  }
  return { hidden, edited, added };
}

export function readWefaxFreq(storage: Store | null = browserStorage()): WefaxFreqState {
  try {
    return parseWefaxFreq(storage?.getItem(WEFAX_FREQ_KEY) ?? null);
  } catch {
    return EMPTY_STATE;
  }
}

function notify(): void {
  try {
    window.dispatchEvent(new CustomEvent(WEFAX_FREQ_EVENT));
  } catch {
    // fuori dal browser (prove) non c'e' chi ascolta
  }
}

/** Salva lo stato; se non resta nessuna differenza toglie la chiave (come il ripristino). */
export function writeWefaxFreq(state: WefaxFreqState, storage: Store | null = browserStorage()): void {
  try {
    if (state.hidden.length === 0 && state.added.length === 0 && Object.keys(state.edited).length === 0) storage?.removeItem(WEFAX_FREQ_KEY);
    else storage?.setItem(WEFAX_FREQ_KEY, JSON.stringify({ v: VERSION, ...state }));
  } catch {
    // archivio pieno o negato: la modifica vale fino alla chiusura
  }
  notify();
}

/** «Ripristina quelle di fabbrica»: torna tutto com'era, anche le voci nascoste. */
export function resetWefaxFreq(storage: Store | null = browserStorage()): void {
  try {
    storage?.removeItem(WEFAX_FREQ_KEY);
  } catch {
    // niente da togliere
  }
  notify();
}

function toPreset(id: string, e: WefaxEntry, ioc = WEFAX_DEFAULT_IOC, lpm = WEFAX_DEFAULT_LPM): WefaxPreset {
  const p: WefaxPreset = {
    id,
    call: e.call,
    place: e.place,
    khz: e.khz,
    dialHz: Math.round(e.khz * 1000) - WEFAX_CARRIER_OFFSET_HZ,
    band: Math.floor(e.khz / 1000),
    ioc,
    lpm,
  };
  return e.band ? { ...p, bandLabel: e.band } : p;
}

/** L'elenco che si usa: quello di fabbrica senza le voci nascoste, con quelle cambiate, piu' quelle aggiunte; ordinato per frequenza. */
export function effectivePresets(state: WefaxFreqState): WefaxPreset[] {
  const hidden = new Set(state.hidden);
  const out: WefaxPreset[] = [];
  for (const p of wefaxPresets()) {
    if (hidden.has(p.id)) continue;
    const e = state.edited[p.id];
    out.push(e ? toPreset(p.id, e, p.ioc, p.lpm) : p);
  }
  for (const a of state.added) out.push(toPreset(a.id, a));
  return out.sort((a, b) => a.khz - b.khz || a.call.localeCompare(b.call));
}

export const isFactoryId = (id: string): boolean => wefaxPresets().some((p) => p.id === id);

/** L'etichetta del gruppo di una voce: quella scelta o «<n> MHz» (sotto 1 MHz: «< 1 MHz»). */
export function bandName(p: Pick<WefaxPreset, 'band' | 'bandLabel'>): string {
  return p.bandLabel ?? (p.band < 1 ? '< 1 MHz' : `${p.band} MHz`);
}

export type WefaxEntryError = 'khz' | 'call' | 'place' | 'band' | 'dup';

/** Controlla una voce prima di salvarla; `selfId` e' la voce che si sta cambiando (non conta come doppione di se stessa). */
export function validateEntry(
  e: { call: string; place: string; khz: string | number; band?: string },
  list: readonly WefaxPreset[],
  selfId: string | null = null,
): WefaxEntryError | null {
  const khz = typeof e.khz === 'number' ? e.khz : Number(String(e.khz).trim().replace(',', '.'));
  if (String(e.khz).trim() === '' || !Number.isFinite(khz) || khz < KHZ_MIN || khz > KHZ_MAX) return 'khz';
  const call = e.call.trim();
  if (call === '' || call.length > CALL_MAX) return 'call';
  if (e.place.trim().length > PLACE_MAX) return 'place';
  if ((e.band ?? '').trim().length > BAND_MAX) return 'band';
  if (list.some((p) => p.id !== selfId && p.khz === khz && p.call.toLowerCase() === call.toLowerCase())) return 'dup';
  return null;
}

/** La voce pulita (testi senza spazi ai lati, kHz numerici con virgola o punto). Da chiamare dopo `validateEntry`. */
export function normalizeEntry(e: { call: string; place: string; khz: string | number; band?: string }): WefaxEntry {
  const khz = typeof e.khz === 'number' ? e.khz : Number(String(e.khz).trim().replace(',', '.'));
  const band = (e.band ?? '').trim();
  return band ? { call: e.call.trim(), place: e.place.trim(), khz, band } : { call: e.call.trim(), place: e.place.trim(), khz };
}

/** Aggiunge (`id` null) o cambia una voce. */
export function saveEntry(state: WefaxFreqState, id: string | null, e: WefaxEntry): WefaxFreqState {
  if (id === null) {
    let n = state.added.length + 1;
    while (state.added.some((a) => a.id === `u:${n}`)) n++;
    return { ...state, added: [...state.added, { id: `u:${n}`, ...e }] };
  }
  if (isFactoryId(id)) return { ...state, edited: { ...state.edited, [id]: e } };
  return {
    ...state,
    added: state.added.map((a) => (a.id === id ? { id, ...e } : a)),
  };
}

/** Cancella una voce: quelle di fabbrica si nascondono (tornano con il ripristino), quelle aggiunte si tolgono. */
export function removeEntry(state: WefaxFreqState, id: string): WefaxFreqState {
  if (isFactoryId(id)) {
    const { [id]: _gone, ...edited } = state.edited;
    return {
      ...state,
      hidden: state.hidden.includes(id) ? state.hidden : [...state.hidden, id],
      edited,
    };
  }
  return { ...state, added: state.added.filter((a) => a.id !== id) };
}

/** Le frequenze dell'utente per il menu della pagina WEFAX. */
export const userWefaxPresets = (): WefaxPreset[] => effectivePresets(readWefaxFreq());
export const userWefaxGroups = (): WefaxGroup[] => wefaxGroups(userWefaxPresets());
export const userPresetAt = (dialHz: number): WefaxPreset | null => presetAt(dialHz, userWefaxPresets());

function subscribe(cb: () => void): () => void {
  const onStorage = (e: StorageEvent) => {
    if (e.key === null || e.key === WEFAX_FREQ_KEY) cb();
  };
  window.addEventListener(WEFAX_FREQ_EVENT, cb);
  window.addEventListener('storage', onStorage);
  return () => {
    window.removeEventListener(WEFAX_FREQ_EVENT, cb);
    window.removeEventListener('storage', onStorage);
  };
}

const snapshot = (): string => {
  try {
    return browserStorage()?.getItem(WEFAX_FREQ_KEY) ?? '';
  } catch {
    return '';
  }
};

/** Lo stato attuale, aggiornato quando cambia (anche da un'altra finestra). */
export function useWefaxFreq(): WefaxFreqState {
  const text = useSyncExternalStore(subscribe, snapshot, () => '');
  return parseWefaxFreq(text || null);
}
