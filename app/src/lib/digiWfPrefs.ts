import { create } from 'zustand';
import { jtdxPaletteLut } from '../render/colormap';
import { JTDX_PALETTES } from '../render/wfPalettes';
import { browserStorage } from './theme';

/** «Classica»: i colori che la cascata dei modi testuali ha sempre avuto (blu, azzurro, verde, giallo, rosso). */
export const DIGI_CLASSIC = 'Classic';
export const DIGI_WF_LIMITS = { gain: [-50, 50], zero: [-50, 50] } as const;
const KEY = 'aurora-digi-wf';

/**
 * Colori della cascata dei modi testuali (CW, PSK, RTTY...): la tavolozza (una di quelle di JTDX o la «Classica»), il guadagno e lo zero
 * (-50...50, come nella cascata dell'FT8). Sono scelte solo di questa cascata: non toccano quelle dell'FT8.
 */
export interface DigiWfPrefs {
  palette: string;
  gain: number;
  zero: number;
}

export const DEFAULT_DIGI_WF: DigiWfPrefs = { palette: DIGI_CLASSIC, gain: 0, zero: 0 };

function clamp(key: 'gain' | 'zero', value: unknown): number {
  if (typeof value !== 'number' || !Number.isFinite(value)) return 0;
  const [lo, hi] = DIGI_WF_LIMITS[key];
  return Math.min(hi, Math.max(lo, Math.round(value)));
}

export function isDigiPalette(name: unknown): name is string {
  return typeof name === 'string' && (name === DIGI_CLASSIC || Object.prototype.hasOwnProperty.call(JTDX_PALETTES, name));
}

/** La scelta salvata; un valore illeggibile o sconosciuto riporta a quello di fabbrica. */
export function readDigiWf(storage: Pick<Storage, 'getItem'> | null): DigiWfPrefs {
  try {
    const raw = storage?.getItem(KEY);
    if (!raw) return { ...DEFAULT_DIGI_WF };
    const p = JSON.parse(raw) as Record<string, unknown>;
    return { palette: isDigiPalette(p.palette) ? p.palette : DIGI_CLASSIC, gain: clamp('gain', p.gain), zero: clamp('zero', p.zero) };
  } catch {
    return { ...DEFAULT_DIGI_WF };
  }
}

export function saveDigiWf(storage: Pick<Storage, 'setItem'> | null, prefs: DigiWfPrefs): void {
  try {
    storage?.setItem(KEY, JSON.stringify(prefs));
  } catch {
    // senza archivio la scelta non si ricorda, ma la cascata funziona lo stesso
  }
}

interface DigiWfState {
  prefs: DigiWfPrefs;
  set(change: Partial<DigiWfPrefs>): void;
}

export const useDigiWf = create<DigiWfState>((set, get) => ({
  prefs: readDigiWf(browserStorage()),
  set(change) {
    const next = { ...get().prefs, ...change };
    const prefs: DigiWfPrefs = { palette: isDigiPalette(next.palette) ? next.palette : DIGI_CLASSIC, gain: clamp('gain', next.gain), zero: clamp('zero', next.zero) };
    saveDigiWf(browserStorage(), prefs);
    set({ prefs });
  },
}));

/** Il colore della «Classica» per un valore 0-255: blu scuro, azzurro, verde, giallo, rosso. */
export function classicColor(v: number): [number, number, number] {
  const x = v / 255;
  if (x < 0.25) return [0, 0, Math.round(60 + x * 4 * 150)];
  if (x < 0.5) return [0, Math.round((x - 0.25) * 4 * 255), 210];
  if (x < 0.75) return [Math.round((x - 0.5) * 4 * 255), 255, Math.round(210 * (1 - (x - 0.5) * 4))];
  return [255, Math.round(255 * (1 - (x - 0.75) * 4)), 0];
}

/**
 * La tabella da 256 colori della cascata: per ogni valore 0-255 della riga il colore, dopo la tavolozza scelta, il guadagno
 * (`10^(0,02 * gain)`, come nella cascata dell'FT8: alzato i segnali diventano piu' vivi) e lo zero (sposta tutta la scala: alzato schiarisce
 * anche il fondo). Con la «Classica» e guadagno e zero a 0 e' esattamente la tabella di sempre.
 */
export function digiWfTable(prefs: DigiWfPrefs): Array<[number, number, number]> {
  const points = prefs.palette === DIGI_CLASSIC ? null : JTDX_PALETTES[prefs.palette];
  const lut = points ? jtdxPaletteLut(points) : null;
  const factor = 10 ** (0.02 * prefs.gain);
  const shift = prefs.zero * 2.5;
  return Array.from({ length: 256 }, (_, v) => {
    const i = Math.min(255, Math.max(0, Math.round(v * factor + shift)));
    if (!lut) return classicColor(i);
    return [lut[i * 4]!, lut[i * 4 + 1]!, lut[i * 4 + 2]!];
  });
}
