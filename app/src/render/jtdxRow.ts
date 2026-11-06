/**
 * La riga della cascata come la calcola il Wide Graph di JTDX 2.2.159 (`WideGraph::dataSink2`, `CPlotter::draw`, `flat4.f90`).
 * Il core (`audio-spectrum`) manda l'ampiezza di `symspec.f90` di ogni canale da 0,732 Hz, in dB su un byte; qui si fa
 * quello che fa JTDX da li' in poi: media di N righe, somma di `Bins/Pixel` canali, scala per la mediana, dB (con o senza
 * «Flatten»), poi il colore con guadagno e zero (nello shader). Indirizzi e formule: `docs/analisi-jtdx-reverse-rea.md`.
 */

/** Gli stessi valori del core (`core/crates/audio-spectrum/src/analyzer.rs`): vanno cambiati insieme. */
export const DB_MIN = -40;
export const DB_STEP = 80 / 255;

/** `fac2 = 1000 * gain` con `gain = 0.0063` in `symspec.f90`: da `sx` a `s(i)`, l'unita' del Wide Graph. */
export const FAC2 = 1000 * 0.0063;

/** Il livello `y` (dB del Wide Graph) si salva in un byte con passo 0,25 dB da -24 dB (fino a +39 dB). */
export const Y_MIN = -24;
export const Y_PER_DB = 4;
export const CODE_MAX = 252;

const AMPLITUDE = (() => {
  const table = new Float32Array(256);
  for (let i = 0; i < 256; i++) table[i] = 10 ** ((DB_MIN + i * DB_STEP) / 20);
  return table;
})();

/** Ampiezza lineare (unita' di `sx`) di un byte del core. */
export function amplitudeOfByte(byte: number): number {
  return AMPLITUDE[byte & 255]!;
}

export function codeOfY(y: number): number {
  if (!Number.isFinite(y)) return 0;
  return Math.min(CODE_MAX, Math.max(0, Math.round((y - Y_MIN) * Y_PER_DB)));
}

export function yOfCode(code: number): number {
  return code / Y_PER_DB + Y_MIN;
}

/** Il colore: `y1 = 10 * gain * y + 10 * zero + 40`, tenuto in 0-254 (`CPlotter::draw`); `gain` e' `10^(0,02 * Gain)`. */
export function paletteIndex(y: number, gainSlider: number, zeroSlider: number): number {
  const y1 = 10 * 10 ** (0.02 * gainSlider) * y + 10 * zeroSlider + 40;
  return Math.min(254, Math.max(0, Math.trunc(y1)));
}

/** Le righe che il core manda ogni secondo (`ROWS_PER_SECOND` di `core/crates/audio-spectrum/src/analyzer.rs`). */
export const CORE_ROWS_PER_SECOND = 15;

/**
 * Il ritmo della cascata: raccoglie le righe del core (1/15 s ciascuna) e restituisce la loro media ogni `rowSecs * navg`
 * secondi. Con 1/15 s e' la media di N righe di JTDX; con 0,288 s (mezzo simbolo FT8, il passo di `symspec.f90` di WSJT-X)
 * ogni riga raccoglie 4 o 5 righe del core, con l'avanzo portato avanti, cosi' un periodo di 15 s fa ~52 righe come WSJT-X.
 */
export class RowPacer {
  private sum: Float32Array | null = null;
  private count = 0;
  private elapsed = 0;

  constructor(private readonly coreRowSecs = 1 / CORE_ROWS_PER_SECOND) {}

  /** Una riga del core; la media (unita' di `sx`) quando la riga della cascata e' completa, altrimenti null. */
  add(row: Uint8Array, rowSecs: number, navg: number): Float32Array | null {
    if (row.length === 0) return null;
    if (!this.sum || this.sum.length !== row.length) {
      this.sum = new Float32Array(row.length);
      this.count = 0;
      this.elapsed = 0;
    }
    for (let i = 0; i < row.length; i++) this.sum[i]! += AMPLITUDE[row[i]!]!;
    this.count++;
    this.elapsed += this.coreRowSecs;
    const step = Math.max(this.coreRowSecs, rowSecs * Math.max(1, navg));
    // tolleranza per gli errori di arrotondamento (15 righe da 1/15 s devono fare 1 s esatto)
    if (this.elapsed + 1e-9 < step) return null;
    const out = new Float32Array(row.length);
    for (let i = 0; i < row.length; i++) out[i] = this.sum[i]! / this.count;
    this.sum.fill(0);
    this.count = 0;
    this.elapsed = Math.min(this.elapsed - step, step);
    return out;
  }
}

export interface ColumnOptions {
  /** Canali sommati in un pixel (1-100). */
  binsPerPixel: number;
  /** Frequenza del bordo sinistro e del bordo destro (Hz). */
  startHz: number;
  endHz: number;
  /** «Scale»: divide per la mediana (la scala del Wide Graph). */
  scale: boolean;
  /** «Flatten»: toglie la linea di base del rumore. */
  flatten: boolean;
}

/** Media di N righe (`N Avg`): la somma delle ampiezze lineari, normalizzata a ogni N-esima riga (`dataSink2`). */
export class RowAverager {
  private sum: Float32Array | null = null;
  private count = 0;

  /** Accumula una riga di byte del core; ogni `every` righe restituisce l'ampiezza media (unita' di `sx`), altrimenti null. */
  add(row: Uint8Array, every: number): Float32Array | null {
    if (row.length === 0) return null;
    if (!this.sum || this.sum.length !== row.length) {
      this.sum = new Float32Array(row.length);
      this.count = 0;
    }
    for (let i = 0; i < row.length; i++) this.sum[i]! += AMPLITUDE[row[i]!]!;
    this.count++;
    if (this.count < Math.max(1, every)) return null;
    const out = new Float32Array(row.length);
    for (let i = 0; i < row.length; i++) out[i] = this.sum[i]! / this.count;
    this.sum.fill(0);
    this.count = 0;
    return out;
  }
}

/** Media dall'inizio del periodo (`savg = ssum / ihsym`): serve allo spettro «Cumulative». */
export class PeriodAverage {
  private sum: Float32Array | null = null;
  private count = 0;

  add(row: Uint8Array): void {
    if (row.length === 0) return;
    if (!this.sum || this.sum.length !== row.length) {
      this.sum = new Float32Array(row.length);
      this.count = 0;
    }
    for (let i = 0; i < row.length; i++) this.sum[i]! += AMPLITUDE[row[i]!]!;
    this.count++;
  }

  reset(): void {
    this.sum?.fill(0);
    this.count = 0;
  }

  /** Ampiezza media per canale, o null se il periodo e' appena cominciato. */
  average(): Float32Array | null {
    if (!this.sum || this.count === 0) return null;
    const out = new Float32Array(this.sum.length);
    for (let i = 0; i < out.length; i++) out[i] = this.sum[i]! / this.count;
    return out;
  }
}

/** Numero di pixel della riga: da `startHz` a `endHz`, `binsPerPixel` canali ciascuno. */
export function columnCount(binWidthHz: number, o: Pick<ColumnOptions, 'binsPerPixel' | 'startHz' | 'endHz'>): number {
  const bpp = Math.max(1, Math.round(o.binsPerPixel));
  if (!(binWidthHz > 0)) return 0;
  return Math.max(0, Math.floor((o.endHz - o.startHz) / (bpp * binWidthHz)));
}

/** `swide[j]`: somma di `binsPerPixel` canali (ampiezze lineari), a partire dal canale di `startHz` (`dataSink2`). */
export function sumColumns(amplitude: Float32Array, binWidthHz: number, o: Pick<ColumnOptions, 'binsPerPixel' | 'startHz' | 'endHz'>, factor = FAC2): Float32Array {
  const bpp = Math.max(1, Math.round(o.binsPerPixel));
  const iz = columnCount(binWidthHz, o);
  const out = new Float32Array(iz);
  let i = Math.floor(o.startHz / binWidthHz + 0.5);
  for (let j = 0; j < iz; j++) {
    let sum = 0;
    for (let k = 0; k < bpp; k++) {
      const v = amplitude[i++];
      if (v !== undefined) sum += v;
    }
    out[j] = sum * factor;
  }
  return out;
}

/** `scale_by_median` di `plotter.cpp`: divide per la mediana (indice `n/2` dei valori ordinati) piu' 0,01. */
export function scaleByMedian(values: Float32Array): void {
  if (values.length === 0) return;
  const sorted = Float32Array.from(values).sort();
  const median = sorted[Math.floor(values.length / 2)]! + 0.01;
  if (median <= 0) return;
  for (let i = 0; i < values.length; i++) values[i]! /= median;
}

/** Il percentile di `pctile.f90`: l'elemento `nint(n * pct / 100)` (minimo il primo) dei valori ordinati. */
export function percentile(values: ArrayLike<number>, pct: number): number {
  const sorted = Float64Array.from(values as ArrayLike<number>).sort();
  const j = Math.max(1, Math.round(sorted.length * 0.01 * pct));
  return sorted[Math.min(sorted.length, j) - 1] ?? 0;
}

/** Soluzione ai minimi quadrati di un polinomio di `terms` coefficienti (eliminazione di Gauss con pivot); null se singolare. */
function polyFit(x: number[], y: number[], terms: number): number[] | null {
  const m = terms;
  const a: number[][] = Array.from({ length: m }, () => new Array<number>(m + 1).fill(0));
  for (let k = 0; k < x.length; k++) {
    const powers = [1];
    for (let p = 1; p < 2 * m; p++) powers.push(powers[p - 1]! * x[k]!);
    for (let r = 0; r < m; r++) {
      for (let c = 0; c < m; c++) a[r]![c]! += powers[r + c]!;
      a[r]![m]! += powers[r]! * y[k]!;
    }
  }
  for (let col = 0; col < m; col++) {
    let pivot = col;
    for (let r = col + 1; r < m; r++) if (Math.abs(a[r]![col]!) > Math.abs(a[pivot]![col]!)) pivot = r;
    if (Math.abs(a[pivot]![col]!) < 1e-12) return null;
    [a[col], a[pivot]] = [a[pivot]!, a[col]!];
    for (let r = col + 1; r < m; r++) {
      const f = a[r]![col]! / a[col]![col]!;
      for (let c = col; c <= m; c++) a[r]![c]! -= f * a[col]![c]!;
    }
  }
  const coef = new Array<number>(m).fill(0);
  for (let r = m - 1; r >= 0; r--) {
    let sum = a[r]![m]!;
    for (let c = r + 1; c < m; c++) sum -= a[r]![c]! * coef[c]!;
    coef[r] = sum / a[r]![r]!;
  }
  return coef;
}

const FLAT_SEGMENTS = 10;
const FLAT_PERCENT = 10;
const FLAT_POINTS = 1000;
const FLAT_TERMS = 5;

/**
 * `flat4.f90`: da potenza lineare a dB (`10 * log10`) e, con `flatten`, toglie la linea di base: per ognuno dei 10 segmenti si
 * tengono i punti sotto il 10 percentile (al massimo 1000), si adatta un polinomio di 4 grado e lo si sottrae.
 */
export function flat4(values: Float32Array, flatten: boolean): void {
  const npts = values.length;
  if (npts === 0) return;
  for (let i = 0; i < npts; i++) values[i] = 10 * Math.log10(Math.max(values[i]!, 1e-30));
  if (!flatten || npts < FLAT_SEGMENTS) return;
  const nlen = Math.floor(npts / FLAT_SEGMENTS);
  const i0 = Math.floor(npts / 2);
  const xs: number[] = [];
  const ys: number[] = [];
  for (let n = 1; n <= FLAT_SEGMENTS; n++) {
    let ib = n * nlen;
    const ia = ib - nlen + 1;
    if (n === FLAT_SEGMENTS) ib = npts;
    const base = percentile(values.subarray(ia - 1, ib), FLAT_PERCENT);
    for (let i = ia; i <= ib; i++) {
      if (values[i - 1]! <= base) {
        // come in Fortran: arrivati a 1000 punti si sovrascrive l'ultimo
        if (xs.length < FLAT_POINTS) {
          xs.push(i - i0);
          ys.push(values[i - 1]!);
        } else {
          xs[FLAT_POINTS - 1] = i - i0;
          ys[FLAT_POINTS - 1] = values[i - 1]!;
        }
      }
    }
  }
  // la variabile si porta a [-1, 1] per tenere stabile il sistema; il polinomio e' lo stesso
  const scale = Math.max(1, npts / 2);
  const coef = polyFit(
    xs.map((x) => x / scale),
    ys,
    FLAT_TERMS,
  );
  if (!coef) return;
  for (let i = 1; i <= npts; i++) {
    const u = (i - i0) / scale;
    let fit = 0;
    for (let k = FLAT_TERMS - 1; k >= 0; k--) fit = fit * u + coef[k]!;
    values[i - 1]! -= fit;
  }
}

/** Il valore piu' basso (0 se vuoto): il fondo per le colonne senza dati. */
function floorOf(values: Float32Array): number {
  let min = Infinity;
  for (const v of values) if (v < min) min = v;
  return Number.isFinite(min) ? min : 0;
}

export interface ProcessedRow {
  /** Livello `y` di ogni pixel (dB del Wide Graph). */
  y: Float32Array;
  /** `y` come byte per la texture (`codeOfY`). */
  codes: Uint8Array;
}

/**
 * Quante colonne della riga cadono su canali che il core ha mandato davvero: come `izVisible` di `CPlotter::draw` (WSJT-X
 * v3.3.0-beta5, `widgets/plotter.cpp`, GPL), che limita l'appiattimento al tratto visibile con dati fino a 5000 Hz.
 */
export function visibleColumns(channels: number, binWidthHz: number, o: Pick<ColumnOptions, 'binsPerPixel' | 'startHz' | 'endHz'>): number {
  const bpp = Math.max(1, Math.round(o.binsPerPixel));
  const first = Math.floor(o.startHz / binWidthHz + 0.5);
  const fit = Math.floor((channels - first) / bpp);
  return Math.max(0, Math.min(columnCount(binWidthHz, o), fit));
}

/**
 * Dalle ampiezze medie di una riga ai pixel della cascata: somma dei canali, scala, dB, flatten. Scala e appiattimento si
 * calcolano solo sul tratto visibile con dati (WSJT-X `plotter.cpp`: `flat4_(swidePlot, &izVisible, ...)`), cosi' un bordo
 * senza canali non piega la linea di base.
 */
export function processRow(amplitude: Float32Array, binWidthHz: number, o: ColumnOptions): ProcessedRow {
  const y = sumColumns(amplitude, binWidthHz, o);
  const visible = y.subarray(0, visibleColumns(amplitude.length, binWidthHz, o));
  if (o.scale) scaleByMedian(visible);
  flat4(visible, o.flatten);
  // le colonne oltre i dati restano al fondo della scala
  y.fill(floorOf(visible), visible.length);
  const codes = new Uint8Array(y.length);
  for (let i = 0; i < y.length; i++) codes[i] = codeOfY(y[i]!);
  return { y, codes };
}

/**
 * La curva «Cumulative» dello spettro: la media del periodo (ampiezza per canale) scalata per la mediana dei canali visibili e
 * portata in dB come in `CPlotter::draw` (`scale_by_median` e `flat4` su `savg`), poi la media dei dB di `binsPerPixel` canali.
 */
export function cumulativeCurve(average: Float32Array, binWidthHz: number, o: ColumnOptions): Float32Array {
  const bpp = Math.max(1, Math.round(o.binsPerPixel));
  const iz = columnCount(binWidthHz, o);
  const first = Math.floor(o.startHz / binWidthHz + 0.5);
  // come `savgBins` di WSJT-X `plotter.cpp`: solo i canali visibili che esistono (`NSMAX - j0`)
  const bins = new Float32Array(Math.max(0, Math.min(iz * bpp, average.length - first)));
  for (let i = 0; i < bins.length; i++) bins[i] = average[first + i]!;
  if (o.scale) scaleByMedian(bins);
  flat4(bins, o.flatten);
  const out = new Float32Array(iz);
  const full = Math.floor(bins.length / bpp);
  for (let j = 0; j < full; j++) {
    let sum = 0;
    for (let k = 0; k < bpp; k++) sum += bins[j * bpp + k]!;
    out[j] = sum / bpp;
  }
  out.fill(floorOf(out.subarray(0, full)), full);
  return out;
}

/**
 * L'ordinata della curva dello spettro (`CPlotter::draw`): 70 unita' di livello fanno l'altezza; con «Current» il minimo della
 * riga e' il fondo, con «Cumulative» no; senza «Flatten» la curva sale di 15 (`+= 15`). Restituisce frazioni di altezza dall'alto.
 */
export function spectrumCurve(
  values: Float32Array,
  mode: 'current' | 'cumulative',
  gain2d: number,
  zero2d: number,
  flatten: boolean,
): Float32Array {
  const gain = 10 ** (0.02 * gain2d);
  let min = Infinity;
  for (const v of values) if (v < min) min = v;
  const out = new Float32Array(values.length);
  for (let i = 0; i < values.length; i++) {
    let y2 = mode === 'current' ? gain * (values[i]! - min) + zero2d : gain * (values[i]! + zero2d);
    if (!flatten) y2 += 15;
    out[i] = 0.9 - y2 / 70;
  }
  return out;
}

/**
 * Fa «salire» lo spettro quando il segnale e' debole: il fondo (il punto piu' basso) resta dov'e' e la curva si allarga verso l'alto
 * finche' i picchi arrivano a circa il 60% dell'altezza (al massimo 5 volte), come negli altri programmi che regolano da soli il guadagno.
 * Con un segnale gia' alto non cambia niente.
 */
export function liftCurve(curve: Float32Array): Float32Array {
  if (curve.length < 2) return curve;
  let base = -Infinity;
  let peak = Infinity;
  for (const v of curve) {
    if (v > base) base = v;
    if (v < peak) peak = v;
  }
  const span = base - peak;
  if (!(span > 0)) return curve;
  const k = Math.min(5, 0.6 / span);
  if (k <= 1) return curve;
  const out = new Float32Array(curve.length);
  for (let i = 0; i < curve.length; i++) out[i] = base - (base - curve[i]!) * k;
  return out;
}

/** Porta una riga alla larghezza della texture: se ha piu' colonne tiene il massimo di ognuna (un segnale stretto non sparisce). */
export function fitWidth(codes: Uint8Array, width: number): Uint8Array {
  if (codes.length <= width) return codes;
  const out = new Uint8Array(width);
  for (let i = 0; i < width; i++) {
    const a = Math.floor((i * codes.length) / width);
    const b = Math.max(a + 1, Math.floor(((i + 1) * codes.length) / width));
    let max = 0;
    for (let k = a; k < b; k++) if (codes[k]! > max) max = codes[k]!;
    out[i] = max;
  }
  return out;
}
