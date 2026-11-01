import { create } from 'zustand';
import { DEFAULT_PALETTE, JTDX_PALETTES } from '../render/wfPalettes';
import { browserStorage } from './theme';

/** Un colore della tavolozza personalizzata: [r, g, b], 0-255. */
export type PaletteColor = readonly [number, number, number];

/**
 * Le opzioni del Wide Graph di JTDX 2.2.159 (`WideGraph::WideGraph`, gruppo `[WideGraph]` del `JTDX.ini`), con i valori di fabbrica
 * e i limiti di JTDX: Bins/Pixel 5 (1-100), Start 0 Hz (0-4900), N Avg 1 (1-50), tavolozza «Default», Scale acceso, Flatten spento,
 * Gain e Zero 0 (-50...50), Spec 30 % (0-100), «Current», Bars acceso, Freq spento.
 */
export interface WfPrefs {
  binsPerPixel: number;
  startHz: number;
  navg: number;
  /** Nome di una tavolozza di JTDX o `User` per quella personalizzata. */
  palette: string;
  userPalette: PaletteColor[];
  scale: boolean;
  flatten: boolean;
  gain: number;
  zero: number;
  /** Quanto dell'altezza prende lo spettro (%); 0 lo nasconde. */
  spectrumPercent: number;
  spectrumMode: 'current' | 'cumulative';
  gain2d: number;
  zero2d: number;
  /** «Bars»: le due barre verticali Rx e Tx sulla cascata. */
  bars: boolean;
  /** «Freq»: la frequenza sotto il puntatore. */
  freq: boolean;
}

export const WF_LIMITS = {
  binsPerPixel: [1, 100],
  startHz: [0, 4900],
  navg: [1, 50],
  gain: [-50, 50],
  zero: [-50, 50],
  spectrumPercent: [0, 100],
  gain2d: [-50, 50],
  zero2d: [-50, 50],
} as const;

export const USER_PALETTE = 'User';
const KEY = 'aurora-wf-prefs';

type ReadableStorage = Pick<Storage, 'getItem'> | null;
type WritableStorage = Pick<Storage, 'setItem'> | null;

export function defaultWfPrefs(): WfPrefs {
  return {
    binsPerPixel: 5,
    startHz: 0,
    navg: 1,
    palette: DEFAULT_PALETTE,
    userPalette: JTDX_PALETTES[DEFAULT_PALETTE]!.map((c) => [...c] as unknown as PaletteColor),
    scale: true,
    flatten: false,
    gain: 0,
    zero: 0,
    spectrumPercent: 30,
    spectrumMode: 'current',
    gain2d: 0,
    zero2d: 0,
    bars: true,
    freq: false,
  };
}

type NumericKey = keyof typeof WF_LIMITS;

/** Come JTDX: un valore fuori dai limiti torna a quello di fabbrica (non viene tagliato). */
function inRange(key: NumericKey, value: unknown, fallback: number): number {
  if (typeof value !== 'number' || !Number.isFinite(value)) return fallback;
  const n = Math.round(value);
  const [lo, hi] = WF_LIMITS[key];
  return n >= lo && n <= hi ? n : fallback;
}

/** Porta un valore ai limiti (per i controlli dell'interfaccia). */
export function clampWf(key: NumericKey, value: number): number {
  const [lo, hi] = WF_LIMITS[key];
  return Number.isFinite(value) ? Math.min(hi, Math.max(lo, Math.round(value))) : defaultWfPrefs()[key];
}

function readPalette(raw: unknown): PaletteColor[] | null {
  if (!Array.isArray(raw) || raw.length < 2 || raw.length > 255) return null;
  const out: PaletteColor[] = [];
  for (const c of raw) {
    if (!Array.isArray(c) || c.length !== 3) return null;
    const rgb = c.map((v) => (typeof v === 'number' && Number.isFinite(v) ? Math.min(255, Math.max(0, Math.round(v))) : -1));
    if (rgb.some((v) => v < 0)) return null;
    out.push([rgb[0]!, rgb[1]!, rgb[2]!]);
  }
  return out;
}

export function readWfPrefs(storage: ReadableStorage): WfPrefs {
  const prefs = defaultWfPrefs();
  try {
    const raw = storage?.getItem(KEY);
    if (!raw) return prefs;
    const parsed = JSON.parse(raw) as Record<string, unknown>;
    for (const key of Object.keys(WF_LIMITS) as NumericKey[]) prefs[key] = inRange(key, parsed[key], prefs[key]);
    for (const key of ['scale', 'flatten', 'bars', 'freq'] as const) {
      if (typeof parsed[key] === 'boolean') prefs[key] = parsed[key];
    }
    if (parsed.spectrumMode === 'current' || parsed.spectrumMode === 'cumulative') prefs.spectrumMode = parsed.spectrumMode;
    const user = readPalette(parsed.userPalette);
    if (user) prefs.userPalette = user;
    if (typeof parsed.palette === 'string' && (parsed.palette === USER_PALETTE || parsed.palette in JTDX_PALETTES)) prefs.palette = parsed.palette;
  } catch {
    return defaultWfPrefs();
  }
  return prefs;
}

export function saveWfPrefs(storage: WritableStorage, prefs: WfPrefs): void {
  try {
    storage?.setItem(KEY, JSON.stringify(prefs));
  } catch {
    // senza archivio le opzioni valgono fino alla chiusura della pagina
  }
}

/** I punti di colore della tavolozza scelta. */
export function paletteColors(prefs: Pick<WfPrefs, 'palette' | 'userPalette'>): ReadonlyArray<PaletteColor> {
  if (prefs.palette === USER_PALETTE) return prefs.userPalette;
  return JTDX_PALETTES[prefs.palette] ?? JTDX_PALETTES[DEFAULT_PALETTE]!;
}

interface WfState {
  prefs: WfPrefs;
  set(change: Partial<WfPrefs>): void;
  reset(): void;
}

export const useWfPrefs = create<WfState>((set, get) => ({
  prefs: readWfPrefs(browserStorage()),
  set(change) {
    const next = { ...get().prefs, ...change };
    saveWfPrefs(browserStorage(), next);
    set({ prefs: next });
  },
  reset() {
    const next = defaultWfPrefs();
    saveWfPrefs(browserStorage(), next);
    set({ prefs: next });
  },
}));
