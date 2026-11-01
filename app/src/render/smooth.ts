/**
 * Orologio dello scorrimento: dice a che punto e' il passaggio da una riga alla successiva (0-1), cosi' la cascata
 * scende in modo continuo invece di saltare di una riga alla volta.
 */
export class RowClock {
  private last = 0;
  private interval: number;

  constructor(initialMs = 67) {
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
