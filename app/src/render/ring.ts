/** Anello di righe della cascata: le righe nuove sovrascrivono le piu' vecchie. */
export class RowRing {
  readonly data: Uint8Array;
  /** Due byte per riga: riga verde di inizio periodo (Rx) e riga di trasmissione, come il `flag` di `pushRow`. */
  readonly flags: Uint8Array;
  /** Indice della prossima riga da scrivere (la piu' recente sta in `head - 1`). */
  head = 0;
  filled = 0;

  constructor(
    readonly width: number,
    readonly height: number,
  ) {
    if (!(width > 0) || !(height > 0)) throw new Error('dimensioni dell\'anello non valide');
    this.data = new Uint8Array(width * height);
    this.flags = new Uint8Array(height * 2);
  }

  /** Scrive una riga (riscalata a `width` se serve) e restituisce l'indice usato, o -1 se vuota. */
  push(row: Uint8Array, flag: 0 | 1 | 2 = 0): number {
    if (row.length === 0) return -1;
    const index = this.head;
    this.flags[index * 2] = flag === 1 ? 255 : 0;
    this.flags[index * 2 + 1] = flag === 2 ? 255 : 0;
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

/**
 * L'anello condiviso della cascata: vive fuori dal renderer, cosi' quando la finestra si ridimensiona o il pannello viene
 * rimontato (telefono/finestre, pannelli affiancati, nuovo canvas) il renderer nuovo riparte con tutte le righe gia'
 * ricevute invece che da una cascata vuota. Come `CPlotter::resizeEvent` di WSJT-X 3.2, che copia la storia nella nuova
 * immagine (widgets/plotter.cpp, GPL-3.0).
 */
const shared = new Map<string, RowRing>();

export function sharedRing(width: number, height: number): RowRing {
  const key = `${width}x${height}`;
  let ring = shared.get(key);
  if (!ring) {
    ring = new RowRing(width, height);
    shared.set(key, ring);
  }
  return ring;
}
