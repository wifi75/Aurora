import { create } from 'zustand';

/** Canale leggero tra il flusso del core e la cascata: le righe non passano da React. */
type Listener = (bins: Uint8Array) => void;

const listeners = new Set<Listener>();

/** Senza il filtro della radio la cascata mostra 0-3000 Hz, come sempre. */
export const DEFAULT_SPAN_HZ = 3000;
const MIN_SPAN_HZ = 1000;
const MAX_SPAN_HZ = 5000;

/**
 * Quanto mostra la cascata, dal filtro della radio: la larghezza piu' un margine per il bordo basso (le radio partono da
 * 100-300 Hz), arrotondata ai 100 Hz. Un filtro 200-4000 Hz (3800 di larghezza) da' 0-4000 Hz.
 */
export function spanFromPassband(passbandHz: number | null | undefined): number {
  if (!passbandHz || passbandHz <= 0) return DEFAULT_SPAN_HZ;
  const span = Math.ceil((passbandHz + 200) / 100) * 100;
  return Math.min(MAX_SPAN_HZ, Math.max(MIN_SPAN_HZ, span));
}

/** L'ampiezza mostrata dalla cascata (Hz, da 0). */
export const useSpectrumSpan = create<{ spanHz: number; set(spanHz: number): void }>((set) => ({
  spanHz: DEFAULT_SPAN_HZ,
  set: (spanHz) => set({ spanHz }),
}));

/** Tiene solo i canali da 0 a `spanHz` di una riga che copre `fMinHz`-`fMaxHz`. */
export function cropBins(bins: Uint8Array, fMinHz: number, fMaxHz: number, spanHz: number): Uint8Array {
  if (fMaxHz <= fMinHz || spanHz >= fMaxHz) return bins;
  const n = Math.max(1, Math.round((bins.length * (spanHz - fMinHz)) / (fMaxHz - fMinHz)));
  return bins.subarray(0, Math.min(bins.length, n));
}

export function registerSpectrum(listener: Listener): () => void {
  listeners.add(listener);
  return () => {
    listeners.delete(listener);
  };
}

export function emitSpectrum(bins: Uint8Array, fMinHz = 0, fMaxHz = DEFAULT_SPAN_HZ): void {
  const row = cropBins(bins, fMinHz, fMaxHz, useSpectrumSpan.getState().spanHz);
  for (const listener of listeners) listener(row);
}
