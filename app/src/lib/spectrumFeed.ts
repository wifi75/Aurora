import { create } from 'zustand';

/** Canale leggero tra il flusso del core e la cascata: le righe non passano da React. Riga intera (0-5000 Hz), con la sua scala in Hz. */
export type SpectrumListener = (bins: Uint8Array, fMinHz: number, fMaxHz: number) => void;
type Listener = SpectrumListener;

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

/**
 * Il tratto di frequenza mostrato: dal bordo sinistro (impostazione «Start» del Wide Graph, 0-4900 Hz) fino all'ampiezza del filtro
 * della radio. Con «Start» vicino al bordo destro il tratto non si riduce sotto i 500 Hz.
 */
export function visibleRange(startHz: number, spanHz: number): { from: number; to: number } {
  const to = Math.min(MAX_SPAN_HZ, Math.max(spanHz, startHz + 1000));
  return { from: Math.min(Math.max(0, startHz), to - 500), to };
}

/** L'ampiezza mostrata dalla cascata (Hz, da 0). */
export const useSpectrumSpan = create<{ spanHz: number; set(spanHz: number): void }>((set) => ({
  spanHz: DEFAULT_SPAN_HZ,
  set: (spanHz) => set({ spanHz }),
}));

export function registerSpectrum(listener: Listener): () => void {
  listeners.add(listener);
  return () => {
    listeners.delete(listener);
  };
}

/** La cascata sceglie da sola il tratto da mostrare (Start e ampiezza): la riga arriva intera. */
export function emitSpectrum(bins: Uint8Array, fMinHz = 0, fMaxHz = DEFAULT_SPAN_HZ): void {
  for (const listener of listeners) listener(bins, fMinHz, fMaxHz);
}
