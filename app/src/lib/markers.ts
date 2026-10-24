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

/**
 * Larghezza in Hz del segnale di un modo, cioe' quanto occupa la barra verde (Rx) e rossa (Tx) sopra la cascata: la frequenza
 * indicata e' quella del tono piu' basso. FT8: 8 toni da 6,25 Hz = 50 Hz; FT4: 4 toni da 20,83 Hz = 83,3 Hz. Per gli altri
 * modi 0: resta la barra sottile.
 */
export function signalBandwidthHz(mode: string | undefined): number {
  if (mode === 'FT8') return 50;
  if (mode === 'FT4') return 83.3;
  return 0;
}
