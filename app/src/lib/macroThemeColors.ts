/**
 * Colori dei tasti delle macro, uno per tema: la scelta fatta con il tema chiaro non cambia il tema scuro e viceversa. Il colore scritto
 * nel file della macro resta il punto di partenza di entrambi i temi; solo la scelta nella finestra «Edit user macro» si salva per il tema in uso.
 */
import { useSyncExternalStore } from 'react';
import { THEME_EVENT, browserStorage, type Theme } from './theme';

export const MACRO_THEME_COLORS_KEY = 'aurora.digi.macrocolors';

type ReadableStorage = Pick<Storage, 'getItem'> | null;
type WritableStorage = Pick<Storage, 'getItem' | 'setItem'> | null;

/** Per tema: nome della macro -> colore (numero 0xRRGGBB) o `null` per «nessun colore». */
export type MacroThemeColors = Partial<Record<Theme, Record<string, number | null>>>;

const isColor = (v: unknown): v is number | null => v === null || (typeof v === 'number' && Number.isInteger(v) && v >= 0 && v <= 0xffffff);

/** Legge i colori salvati; un archivio assente o rovinato vale «nessuna scelta». */
export function readMacroColors(storage: ReadableStorage): MacroThemeColors {
  try {
    const raw = JSON.parse(storage?.getItem(MACRO_THEME_COLORS_KEY) ?? 'null') as unknown;
    if (typeof raw !== 'object' || raw === null) return {};
    const out: MacroThemeColors = {};
    for (const theme of ['dark', 'light'] as const) {
      const part = (raw as Record<string, unknown>)[theme];
      if (typeof part !== 'object' || part === null) continue;
      const map: Record<string, number | null> = {};
      for (const [name, color] of Object.entries(part)) if (isColor(color)) map[name] = color;
      out[theme] = map;
    }
    return out;
  } catch {
    return {};
  }
}

/** Il colore di una macro nel tema dato: la scelta per quel tema, altrimenti quello della macro. */
export function themeColor(store: MacroThemeColors, theme: Theme, name: string, base: number | null): number | null {
  const own = store[theme];
  return own && Object.prototype.hasOwnProperty.call(own, name) ? (own[name] ?? null) : base;
}

/** Salva la scelta di un colore per un tema (l'altro tema non cambia). */
export function saveMacroColor(storage: WritableStorage, theme: Theme, name: string, color: number | null): void {
  try {
    const store = readMacroColors(storage);
    store[theme] = { ...(store[theme] ?? {}), [name]: color };
    storage?.setItem(MACRO_THEME_COLORS_KEY, JSON.stringify(store));
  } catch {
    // senza archivio la scelta vale solo fino alla chiusura
  }
}

/** Il tema in uso adesso (lo legge dalla pagina, come lo imposta `applyTheme`). */
export function currentTheme(): Theme {
  return typeof document !== 'undefined' && document.documentElement.dataset.theme === 'light' ? 'light' : 'dark';
}

const subscribe = (cb: () => void) => {
  window.addEventListener(THEME_EVENT, cb);
  return () => window.removeEventListener(THEME_EVENT, cb);
};

/** Il tema in uso, che si aggiorna quando cambia. */
export function useThemeName(): Theme {
  return useSyncExternalStore(subscribe, currentTheme, () => 'dark');
}

/** I colori salvati dal browser (comodo per i componenti). */
export function readMacroColorsNow(): MacroThemeColors {
  return readMacroColors(browserStorage());
}
