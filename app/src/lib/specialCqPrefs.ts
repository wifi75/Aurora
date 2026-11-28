import { create } from 'zustand';
import { SPECIAL_TAGS, type SpecialTag } from './specialCq';
import { browserStorage } from './theme';

/*
 * Preferenze dei CQ speciali (POTA, SOTA, IOTA, WWFF, QRP; richiesta #115), una per dispositivo come gli altri avvisi.
 * Di fabbrica tutto spento: l'interruttore generale chiude tutto, anche se le etichette per tag partono accese (si vedono solo
 * dopo averlo acceso). La ricerca del parco negli spot POTA non e' qui: la fa il core (`online_info::Settings::pota`).
 */

export type TagFlags = Record<SpecialTag, boolean>;

export interface SpecialCqPrefs {
  /** L'interruttore generale: spento, Aurora non cambia niente. */
  enabled: boolean;
  /** Per ogni tag: mostra l'etichetta nell'attivita' di banda. */
  show: TagFlags;
  /** Per ogni tag: avviso sonoro al primo CQ di un nominativo. */
  sound: TagFlags;
}

export const SPECIAL_CQ_KEY = 'aurora-special-cq';

const flags = (value: boolean): TagFlags => Object.fromEntries(SPECIAL_TAGS.map((t) => [t, value])) as TagFlags;

export function defaultSpecialCq(): SpecialCqPrefs {
  return { enabled: false, show: flags(true), sound: flags(false) };
}

type ReadableStorage = Pick<Storage, 'getItem'> | null;
type WritableStorage = Pick<Storage, 'setItem'> | null;

function readFlags(raw: unknown, fallback: TagFlags): TagFlags {
  const out = { ...fallback };
  if (typeof raw !== 'object' || raw === null) return out;
  for (const t of SPECIAL_TAGS) {
    const v = (raw as Record<string, unknown>)[t];
    if (typeof v === 'boolean') out[t] = v;
  }
  return out;
}

export function readSpecialCq(storage: ReadableStorage): SpecialCqPrefs {
  const base = defaultSpecialCq();
  try {
    const raw = storage?.getItem(SPECIAL_CQ_KEY);
    if (!raw) return base;
    const parsed: unknown = JSON.parse(raw);
    if (typeof parsed !== 'object' || parsed === null) return base;
    const p = parsed as Record<string, unknown>;
    return { enabled: p.enabled === true, show: readFlags(p.show, base.show), sound: readFlags(p.sound, base.sound) };
  } catch {
    return base;
  }
}

export function saveSpecialCq(storage: WritableStorage, prefs: SpecialCqPrefs): void {
  try {
    storage?.setItem(SPECIAL_CQ_KEY, JSON.stringify(prefs));
  } catch {
    // senza archivio la scelta non viene ricordata
  }
}

/** Vero se almeno un tag ha l'avviso sonoro acceso (e l'interruttore generale e' acceso). */
export function anySpecialSound(prefs: SpecialCqPrefs): boolean {
  return prefs.enabled && SPECIAL_TAGS.some((t) => prefs.sound[t]);
}

interface SpecialCqState {
  prefs: SpecialCqPrefs;
  set(next: SpecialCqPrefs): void;
}

export const useSpecialCq = create<SpecialCqState>((set) => ({
  prefs: readSpecialCq(browserStorage()),
  set(next) {
    saveSpecialCq(browserStorage(), next);
    set({ prefs: next });
  },
}));
