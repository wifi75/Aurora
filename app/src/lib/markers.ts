/** Posizione (0-100) di una frequenza audio sulla larghezza della cascata. */
export function leftPercent(hz: number, fMinHz: number, fMaxHz: number): number {
  const frac = (hz - fMinHz) / (fMaxHz - fMinHz);
  return Math.min(1, Math.max(0, frac)) * 100;
}

/** Frequenza (Hz interi) a una posizione 0-1 sulla larghezza della cascata. */
export function hzAt(fraction: number, fMinHz: number, fMaxHz: number): number {
  const frac = Math.min(1, Math.max(0, fraction));
  return Math.round(fMinHz + frac * (fMaxHz - fMinHz));
}
