/** Anello di righe della cascata: le righe nuove sovrascrivono le piu' vecchie. */
export class RowRing {
  readonly data: Uint8Array;
  /** Indice della prossima riga da scrivere (la piu' recente sta in `head - 1`). */
  head = 0;
  filled = 0;

  constructor(
    readonly width: number,
    readonly height: number,
  ) {
    if (!(width > 0) || !(height > 0)) throw new Error('dimensioni dell\'anello non valide');
    this.data = new Uint8Array(width * height);
  }

  /** Scrive una riga (riscalata a `width` se serve) e restituisce l'indice usato, o -1 se vuota. */
  push(row: Uint8Array): number {
    if (row.length === 0) return -1;
    const index = this.head;
    const base = index * this.width;
    if (row.length === this.width) {
      this.data.set(row, base);
    } else {
      for (let i = 0; i < this.width; i++) {
        this.data[base + i] = row[Math.floor((i * row.length) / this.width)]!;
      }
    }
    this.head = (this.head + 1) % this.height;
    this.filled = Math.min(this.filled + 1, this.height);
    return index;
  }

  /** Copia le righe dal piu' recente al piu' vecchio (percorso CPU di riserva e test). */
  copyInto(target: Uint8Array): void {
    for (let n = 0; n < this.height; n++) {
      const src = (((this.head - 1 - n) % this.height) + this.height) % this.height;
      target.set(this.data.subarray(src * this.width, (src + 1) * this.width), n * this.width);
    }
  }
}
