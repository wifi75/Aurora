/** Snap e Seek sulla cascata: trovano i segnali nelle righe dello spettro (0-5000 Hz), come i comandi omonimi. */

export interface SpectrumRow {
  bins: Uint8Array;
  fMinHz: number;
  fMaxHz: number;
}

const binHz = (row: SpectrumRow) => (row.fMaxHz - row.fMinHz) / Math.max(1, row.bins.length);

/** I valori delle righe recenti, mediati: un segnale vero resta, il rumore si spiana. */
export class RowAverager {
  private sum: Float32Array | null = null;
  private count = 0;

  push(row: SpectrumRow, depth = 8): void {
    if (!this.sum || this.sum.length !== row.bins.length) {
      this.sum = new Float32Array(row.bins.length);
      this.count = 0;
    }
    // media mobile esponenziale: le ultime righe pesano di piu'
    const k = this.count === 0 ? 1 : 1 / Math.min(depth, this.count + 1);
    for (let i = 0; i < row.bins.length; i++) this.sum[i] = (this.sum[i] ?? 0) * (1 - k) + (row.bins[i] ?? 0) * k;
    this.count++;
  }

  get ready(): boolean {
    return this.sum !== null && this.count > 0;
  }

  values(): Float32Array {
    return this.sum ?? new Float32Array(0);
  }
}

/** Il segnale piu' forte entro `windowHz` da `hz` (Snap): il punto di sintonia giusto per una stazione. Se non c'e' nulla di netto resta `hz`. */
export function snapToPeak(avg: Float32Array, row: SpectrumRow, hz: number, windowHz = 40): number {
  if (avg.length === 0) return hz;
  const w = binHz(row);
  const from = Math.max(0, Math.floor((hz - windowHz - row.fMinHz) / w));
  const to = Math.min(avg.length - 1, Math.ceil((hz + windowHz - row.fMinHz) / w));
  let best = -1;
  let bestV = -1;
  let total = 0;
  for (let i = from; i <= to; i++) {
    const v = avg[i] ?? 0;
    total += v;
    if (v > bestV) {
      bestV = v;
      best = i;
    }
  }
  const mean = total / Math.max(1, to - from + 1);
  // un picco vero sta bene sopra la media della finestra
  if (best < 0 || bestV < mean + 12) return hz;
  // il centro del picco: i bin vicini, pesati
  let num = 0;
  let den = 0;
  for (let i = Math.max(from, best - 2); i <= Math.min(to, best + 2); i++) {
    const v = Math.max(0, (avg[i] ?? 0) - mean);
    num += v * i;
    den += v;
  }
  const centre = den > 0 ? num / den : best;
  return Math.round((row.fMinHz + (centre + 0.5) * w) * 10) / 10;
}

/** Il prossimo segnale da `fromHz` verso sinistra o destra (Seek); `null` se non ce ne sono entro i limiti. */
export function seekSignal(avg: Float32Array, row: SpectrumRow, fromHz: number, direction: 'left' | 'right', limits: { min: number; max: number }, margin = 14): number | null {
  if (avg.length === 0) return null;
  const w = binHz(row);
  const sorted = Array.from(avg).sort((a, b) => a - b);
  const median = sorted[Math.floor(sorted.length / 2)] ?? 0;
  const threshold = median + margin;
  const step = direction === 'right' ? 1 : -1;
  // si parte un po' piu' in la', per non ritrovare la stazione su cui si e' gia'
  let i = Math.round((fromHz - row.fMinHz) / w) + step * Math.ceil(30 / w);
  while (i >= 0 && i < avg.length) {
    const hz = row.fMinHz + (i + 0.5) * w;
    if (hz < limits.min || hz > limits.max) return null;
    if ((avg[i] ?? 0) >= threshold && (avg[i] ?? 0) >= (avg[i - 1] ?? 0) && (avg[i] ?? 0) >= (avg[i + 1] ?? 0)) return snapToPeak(avg, row, hz, 25);
    i += step;
  }
  return null;
}
