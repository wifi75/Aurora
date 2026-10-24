import { create } from 'zustand';
import { BAND_NAMES } from './format';
import { browserStorage } from './theme';

const KEY = 'aurora-bands';

type ReadableStorage = Pick<Storage, 'getItem'> | null;
type WritableStorage = Pick<Storage, 'setItem'> | null;

export function allBands(): string[] {
  return [...BAND_NAMES];
}

const inOrder = (bands: Iterable<string>) => {
  const wanted = new Set(bands);
  return BAND_NAMES.filter((b) => wanted.has(b));
};

/** Bande scelte per la barra: tutte per impostazione predefinita, mai nessuna. */
export function readBands(storage: ReadableStorage): string[] {
  try {
    const raw = storage?.getItem(KEY);
    if (!raw) return allBands();
    const parsed: unknown = JSON.parse(raw);
    if (!Array.isArray(parsed)) return allBands();
    const chosen = inOrder(parsed.filter((b): b is string => typeof b === 'string'));
    return chosen.length > 0 ? chosen : allBands();
  } catch {
    return allBands();
  }
}

export function saveBands(storage: WritableStorage, bands: string[]): void {
  try {
    storage?.setItem(KEY, JSON.stringify(bands));
  } catch {
    // senza archivio la scelta non viene ricordata, ma l'interfaccia funziona lo stesso
  }
}

/** Accende o spegne una banda; ne resta sempre almeno una. */
export function toggleBand(bands: string[], band: string): string[] {
  const next = bands.includes(band) ? bands.filter((b) => b !== band) : [...bands, band];
  return next.length > 0 ? inOrder(next) : bands;
}

/** La banda in uso si vede sempre, anche se l'utente l'ha tolta dall'elenco. */
export function visibleBands(chosen: string[], current: string | null): string[] {
  return current && !chosen.includes(current) ? inOrder([...chosen, current]) : chosen;
}

interface BandFilterState {
  bands: string[];
  toggle(band: string): void;
  reset(): void;
}

/** Stato condiviso: cambiare le bande nelle impostazioni aggiorna subito la barra della stazione. */
export const useBandFilter = create<BandFilterState>((set, get) => ({
  bands: readBands(browserStorage()),
  toggle(band) {
    const next = toggleBand(get().bands, band);
    saveBands(browserStorage(), next);
    set({ bands: next });
  },
  reset() {
    saveBands(browserStorage(), allBands());
    set({ bands: allBands() });
  },
}));
