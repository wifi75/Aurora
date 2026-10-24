/** Media nel tempo di piu' righe di spettro: meno rumore e una cascata piu' lenta e leggibile. */
export class RowAverager {
  private sum: Float32Array | null = null;
  private count = 0;

  constructor(readonly every: number) {
    if (!(every >= 1)) throw new Error('il numero di righe da mediare non e\' valido');
  }

  /** Accumula una riga; ogni `every` righe restituisce la media (byte 0-255), altrimenti null. */
  add(row: Uint8Array): Uint8Array | null {
    if (row.length === 0) return null;
    if (!this.sum || this.sum.length !== row.length) {
      this.sum = new Float32Array(row.length);
      this.count = 0;
    }
    for (let i = 0; i < row.length; i++) this.sum[i]! += row[i]!;
    this.count++;
    if (this.count < this.every) return null;
    const out = new Uint8Array(row.length);
    for (let i = 0; i < row.length; i++) out[i] = Math.round(this.sum[i]! / this.count);
    this.sum.fill(0);
    this.count = 0;
    return out;
  }
}

/**
 * Orologio dello scorrimento: dice a che punto e' il passaggio da una riga alla successiva (0-1), cosi' la cascata
 * scende in modo continuo invece di saltare di una riga alla volta.
 */
export class RowClock {
  private last = 0;
  private interval: number;

  constructor(initialMs = 267) {
    this.interval = initialMs;
  }

  /** Va chiamato quando arriva una riga nuova. */
  push(nowMs: number): void {
    if (this.last > 0) {
      const dt = nowMs - this.last;
      // l'intervallo si stima con una media mobile; gli scarti enormi (scheda in pausa) si ignorano
      if (dt > 20 && dt < 5000) this.interval = this.interval * 0.8 + dt * 0.2;
    }
    this.last = nowMs;
  }

  /** Frazione di riga gia' scorsa dall'ultima riga arrivata, da 0 a 1. */
  fraction(nowMs: number): number {
    if (this.last === 0) return 0;
    return Math.min(1, Math.max(0, (nowMs - this.last) / this.interval));
  }
}
