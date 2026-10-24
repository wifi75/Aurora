import { create } from 'zustand';
import { browserStorage } from './theme';

/** Come si mostrano i messaggi (scheda Generali del motore): unita', paese, bandiera, distanza, DT. */
export interface Prefs {
  distanceUnit: 'km' | 'mi';
  showCountry: boolean;
  showFlag: boolean;
  showDistance: boolean;
  showDt: boolean;
  /** Il doppio clic su un messaggio risponde a quella stazione (come nel motore); spento seleziona soltanto. */
  doubleClickReply: boolean;
  /** Il tasto Esc ferma la trasmissione e spegne AutoTX e Abilita Tx. */
  escStop: boolean;
  /** Al cambio di banda o di modo si svuota l'attivita' di banda: i messaggi dell'altra banda non servono piu'. */
  clearOnChange: boolean;
  /** Quanti QSO di Log4OM2 mostra la finestra Log (1-200). */
  log4omCount: number;
}

const KEY = 'aurora-prefs';

type ReadableStorage = Pick<Storage, 'getItem'> | null;
type WritableStorage = Pick<Storage, 'setItem'> | null;

export function defaultPrefs(): Prefs {
  return { distanceUnit: 'km', showCountry: true, showFlag: true, showDistance: true, showDt: true, doubleClickReply: true, escStop: true, clearOnChange: true, log4omCount: 20 };
}

export function readPrefs(storage: ReadableStorage): Prefs {
  const prefs = defaultPrefs();
  try {
    const raw = storage?.getItem(KEY);
    if (!raw) return prefs;
    const parsed = JSON.parse(raw) as Record<string, unknown>;
    if (parsed.distanceUnit === 'km' || parsed.distanceUnit === 'mi') prefs.distanceUnit = parsed.distanceUnit;
    for (const key of ['showCountry', 'showFlag', 'showDistance', 'showDt', 'doubleClickReply', 'escStop', 'clearOnChange'] as const) {
      if (typeof parsed[key] === 'boolean') prefs[key] = parsed[key];
    }
    if (typeof parsed.log4omCount === 'number' && Number.isFinite(parsed.log4omCount)) prefs.log4omCount = Math.min(200, Math.max(1, Math.round(parsed.log4omCount)));
  } catch {
    return defaultPrefs();
  }
  return prefs;
}

export function savePrefs(storage: WritableStorage, prefs: Prefs): void {
  try {
    storage?.setItem(KEY, JSON.stringify(prefs));
  } catch {
    // senza archivio le preferenze non vengono ricordate, ma l'interfaccia funziona lo stesso
  }
}

export function formatDistance(km: number, unit: 'km' | 'mi'): string {
  return unit === 'mi' ? `${Math.round(km * 0.621371)} mi` : `${km} km`;
}

interface PrefsState {
  prefs: Prefs;
  set(change: Partial<Prefs>): void;
  reset(): void;
}

export const usePrefs = create<PrefsState>((set, get) => ({
  prefs: readPrefs(browserStorage()),
  set(change) {
    const next = { ...get().prefs, ...change };
    savePrefs(browserStorage(), next);
    set({ prefs: next });
  },
  reset() {
    savePrefs(browserStorage(), defaultPrefs());
    set({ prefs: defaultPrefs() });
  },
}));
