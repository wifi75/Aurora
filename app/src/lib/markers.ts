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
  // Q65-15A ... Q65-300E: 65 toni alla velocita' del periodo per 1, 2, 4, 8, 16
  const q65 = /^Q65-(15|30|60|120|300)([A-E])$/.exec(mode?.toUpperCase() ?? '');
  if (q65?.[1] && q65[2]) {
    const nsps: Record<string, number> = { '15': 1800, '30': 3600, '60': 7200, '120': 16000, '300': 41472 };
    return (65 * 12000 * 2 ** (q65[2].charCodeAt(0) - 65)) / (nsps[q65[1]] ?? 3600);
  }
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
 * Dove comincia il segnale rispetto alla frequenza indicata (Hz). Nei modi FT8, FT4, JT65... la frequenza e' quella del tono piu' basso
 * (0); MSK144 invece e' centrato sulla frequenza (toni a 1000 e 2000 Hz attorno ai 1500): la barra parte 1000 Hz prima. Prima la barra
 * di MSK144 andava da 1500 a 3500 Hz (UN7MBV, 2026-11-10: «la cascata e' rimasta come in FT8»).
 */
export function signalStartOffsetHz(mode: string | undefined): number {
  return mode === 'MSK144' ? -signalBandwidthHz(mode) / 2 : 0;
}

/** Quanto lontano dalla frequenza RX la Finestra RX mostra i messaggi (Hz): ±50 come la finestra «Rx Frequency», MSK144 ±100 (la tolleranza di base). */
export function rxWindowHz(mode: string | undefined): number {
  return mode === 'MSK144' ? 100 : 50;
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
