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
 * Larghezza in Hz delle barre Rx e Tx sopra la cascata, come `CPlotter::DrawOverlay` di JTDX: la distanza tra il primo e l'ultimo tono
 * (la frequenza indicata e' quella del tono piu' basso). FT8 `7 * 12000/1920` = 43,75 Hz (8 toni da 6,25 Hz); FT4 `3 * 12000/576` = 62,5 Hz
 * (4 toni da 20,83 Hz); JT65 `66 * 11025/4096` = 177,7 Hz; JT9 `9 * 12000/6912` = 15,6 Hz; T10 `9 * 4 * 12000/6912` = 62,5 Hz.
 * Per gli altri modi 0: resta la barra sottile.
 */
export function signalBandwidthHz(mode: string | undefined): number {
  switch (mode) {
    case 'FT8':
      return (7 * 12000) / 1920;
    case 'FT4':
      return (3 * 12000) / 576;
    case 'JT65':
      return (66 * 11025) / 4096;
    case 'JT9':
      return (9 * 12000) / 6912;
    case 'JT4':
    case 'JT4A':
      return (4 * 11025) / 2520;
    case 'Q65':
      return (65 * 12000) / 3600;
    case 'MSK144':
      return 2000;
    case 'T10':
      return (9 * 4 * 12000) / 6912;
    default:
      return 0;
  }
}

/**
 * La finestra del Filtro attorno alla frequenza Rx (Hz sotto e sopra), come i segni blu di `DrawOverlay`: FT8 -60/+110 (Hound -290/+340),
 * FT4 -95/+179, JT65 -200/+375, JT9 -50/+65,6, T10 -80/+142,4. Per gli altri modi null (nessun filtro disegnato).
 */
export function filterWindowHz(mode: string | undefined, hound = false): { below: number; above: number } | null {
  switch (mode) {
    case 'FT8':
      return hound ? { below: 290, above: 340 } : { below: 60, above: 110 };
    case 'FT4':
      return { below: 95, above: 179 };
    case 'JT65':
      return { below: 200, above: 375 };
    case 'JT9':
      return { below: 50, above: 65.6 };
    case 'JT4':
      return { below: 50, above: 67.5 };
    case 'T10':
      return { below: 80, above: 142.4 };
    default:
      return null;
  }
}
