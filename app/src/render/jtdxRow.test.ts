import { describe, expect, it } from 'vitest';
import {
  DB_MIN,
  DB_STEP,
  FAC2,
  PeriodAverage,
  RowAverager,
  RowPacer,
  amplitudeOfByte,
  codeOfY,
  columnCount,
  cumulativeCurve,
  fitWidth,
  flat4,
  paletteIndex,
  percentile,
  processRow,
  scaleByMedian,
  spectrumCurve,
  sumColumns,
  yOfCode,
} from './jtdxRow';

const BIN = 12000 / 16384;
const byteOfDb = (db: number) => Math.round((db - DB_MIN) / DB_STEP);

describe('palette index, as CPlotter::draw', () => {
  it('is 40 for y = 0: the median level sits in the dark blue part', () => {
    expect(paletteIndex(0, 0, 0)).toBe(40);
  });

  it('adds 10 per dB and clamps to 0..254', () => {
    expect(paletteIndex(5, 0, 0)).toBe(90);
    expect(paletteIndex(-9, 0, 0)).toBe(0);
    expect(paletteIndex(40, 0, 0)).toBe(254);
  });

  it('Zero shifts by 10 per step, Gain multiplies by 10^(0.02 * Gain)', () => {
    expect(paletteIndex(0, 0, 5)).toBe(90);
    expect(paletteIndex(10, 50, 0)).toBe(254);
    expect(paletteIndex(10, -50, 0)).toBe(40 + 10);
    expect(paletteIndex(10, 0, 0)).toBe(140);
  });
});

describe('level bytes', () => {
  it('round-trips y with 0.25 dB steps', () => {
    for (const y of [-24, -3.5, 0, 7.25, 20, 39]) expect(yOfCode(codeOfY(y))).toBeCloseTo(y, 6);
    expect(codeOfY(500)).toBe(252);
    expect(codeOfY(-500)).toBe(0);
    expect(codeOfY(Number.NaN)).toBe(0);
  });

  it('turns a core byte back into the amplitude of symspec', () => {
    expect(amplitudeOfByte(byteOfDb(0))).toBeCloseTo(1, 1);
    expect(amplitudeOfByte(byteOfDb(20))).toBeCloseTo(10, 0);
    expect(amplitudeOfByte(0)).toBeCloseTo(0.01, 4);
  });
});

describe('N Avg', () => {
  it('averages the linear amplitudes of N rows and emits once every N', () => {
    const avg = new RowAverager();
    const row = (db: number) => Uint8Array.from([byteOfDb(db), byteOfDb(db)]);
    expect(avg.add(row(0), 2)).toBeNull();
    const out = avg.add(row(20), 2)!;
    // media lineare di 1 e 10, non dei dB
    expect(out[0]).toBeCloseTo(5.5, 0);
    expect(avg.add(row(0), 2)).toBeNull();
  });

  it('with N = 1 passes every row', () => {
    expect(new RowAverager().add(Uint8Array.from([100]), 1)).not.toBeNull();
  });

  it('PeriodAverage restarts when asked', () => {
    const p = new PeriodAverage();
    expect(p.average()).toBeNull();
    p.add(Uint8Array.from([byteOfDb(0)]));
    p.add(Uint8Array.from([byteOfDb(20)]));
    expect(p.average()![0]).toBeCloseTo(5.5, 0);
    p.reset();
    expect(p.average()).toBeNull();
  });
});

describe('Bins/Pixel and Start', () => {
  it('has one column per bpp channels from Start to the end', () => {
    expect(columnCount(BIN, { binsPerPixel: 5, startHz: 0, endHz: 3000 })).toBe(Math.floor(3000 / (5 * BIN)));
    expect(columnCount(BIN, { binsPerPixel: 1, startHz: 1000, endHz: 3000 })).toBe(Math.floor(2000 / BIN));
    expect(columnCount(0, { binsPerPixel: 5, startHz: 0, endHz: 3000 })).toBe(0);
  });

  it('sums bpp channels (times fac2) starting at the channel of Start', () => {
    const amp = new Float32Array(100).map((_, i) => i);
    const out = sumColumns(amp, 1, { binsPerPixel: 4, startHz: 8, endHz: 20 });
    expect(out.length).toBe(3);
    expect(out[0]).toBeCloseTo((8 + 9 + 10 + 11) * FAC2, 4);
    expect(out[2]).toBeCloseTo((16 + 17 + 18 + 19) * FAC2, 4);
  });
});

describe('scale by median and flat4', () => {
  it('divides by median + 0.01', () => {
    const v = Float32Array.from([1, 2, 3, 4, 5]);
    scaleByMedian(v);
    expect(v[2]).toBeCloseTo(3 / 3.01, 5);
  });

  it('converts to dB with 10 log10 and, without flatten, keeps the shape', () => {
    const v = Float32Array.from([1, 10, 100]);
    flat4(v, false);
    expect(v[0]).toBeCloseTo(0, 4);
    expect(v[1]).toBeCloseTo(10, 4);
    expect(v[2]).toBeCloseTo(20, 4);
  });

  it('percentile is the element nint(n * pct / 100) of the sorted values', () => {
    expect(percentile([5, 1, 4, 2, 3, 9, 8, 7, 6, 10], 10)).toBe(1);
    expect(percentile([5, 1, 4, 2, 3, 9, 8, 7, 6, 10], 50)).toBe(5);
    expect(percentile([3], 10)).toBe(3);
  });

  it('flatten removes a sloped baseline and keeps the signal', () => {
    const n = 800;
    const v = new Float32Array(n);
    for (let i = 0; i < n; i++) v[i] = 10 ** ((3 + (i / n) * 12) / 10); // fondo che sale di 12 dB
    v[400]! *= 10 ** 1.5; // un segnale a +15 dB
    flat4(v, true);
    const base = [20, 200, 600, 780].map((i) => v[i]!);
    expect(Math.max(...base) - Math.min(...base)).toBeLessThan(1.5);
    expect(v[400]! - v[395]!).toBeGreaterThan(12);
  });
});

describe('processRow', () => {
  it('puts the noise at 0 dB with Scale and the signal above it', () => {
    const amp = new Float32Array(2000).fill(1);
    for (let i = 600; i < 610; i++) amp[i] = 10;
    const row = processRow(amp, 1, { binsPerPixel: 5, startHz: 0, endHz: 2000, scale: true, flatten: false });
    expect(row.y.length).toBe(400);
    expect(Math.abs(row.y[10]!)).toBeLessThan(0.1);
    expect(row.y[121]!).toBeGreaterThan(9);
    expect(yOfCode(row.codes[121]!)).toBeCloseTo(row.y[121]!, 0);
  });

  it('a narrow signal stays narrow: 5 channels per pixel make it one or two pixels, not dozens', () => {
    // un tono di JTDX: 3 canali larghi
    const amp = new Float32Array(2000).fill(1);
    amp[1000] = 4;
    amp[1001] = 12;
    amp[1002] = 4;
    const row = processRow(amp, BIN, { binsPerPixel: 5, startHz: 0, endHz: 1400, scale: true, flatten: false });
    const lit = Array.from(row.y).filter((y) => y > 5).length;
    expect(lit).toBeLessThanOrEqual(2);
  });
});

describe('spectrum curve', () => {
  it('current: minimum is the floor; a signal of 10 dB rises 10/70 of the height; +15 without flatten', () => {
    const c = spectrumCurve(Float32Array.from([0, 10]), 'current', 0, 0, true);
    expect(c[0]).toBeCloseTo(0.9, 5);
    expect(c[1]).toBeCloseTo(0.9 - 10 / 70, 5);
    const g = spectrumCurve(Float32Array.from([0, 10]), 'current', 0, 0, false);
    expect(g[0]).toBeCloseTo(0.9 - 15 / 70, 5);
  });

  it('gain 2d multiplies, zero 2d shifts', () => {
    const c = spectrumCurve(Float32Array.from([0, 10]), 'current', 50, 7, true);
    expect(c[1]).toBeCloseTo(0.9 - (10 * 10 + 7) / 70, 5);
  });

  it('cumulative uses the period average in dB (not relative to the minimum)', () => {
    const amp = new Float32Array(400).fill(1);
    amp[100] = 100;
    const curve = cumulativeCurve(amp, 1, { binsPerPixel: 2, startHz: 0, endHz: 400, scale: true, flatten: false });
    expect(curve.length).toBe(200);
    expect(curve[50]!).toBeGreaterThan(curve[10]! + 8);
  });
});

describe('fitWidth', () => {
  it('keeps a row that fits and max-pools one that is too wide', () => {
    const small = Uint8Array.from([1, 2, 3]);
    expect(fitWidth(small, 10)).toBe(small);
    const wide = new Uint8Array(100);
    wide[37] = 200;
    const out = fitWidth(wide, 10);
    expect(out.length).toBe(10);
    expect(out[3]).toBe(200);
    expect(Math.max(...out)).toBe(200);
  });
});

describe('flatten sul tratto visibile (WSJT-X plotter.cpp, izVisible)', () => {
  it('ignora le colonne oltre i canali ricevuti', async () => {
    const { processRow, visibleColumns } = await import('./jtdxRow');
    const binWidth = 1;
    const amplitude = new Float32Array(1000);
    for (let i = 0; i < amplitude.length; i++) amplitude[i] = 1 + i / 1000;
    const o = { binsPerPixel: 1, startHz: 0, endHz: 1500, scale: false, flatten: true };
    expect(visibleColumns(amplitude.length, binWidth, o)).toBe(1000);
    const row = processRow(amplitude, binWidth, o);
    expect(row.y.length).toBe(1500);
    // la pendenza dei dati veri e' tolta: senza il limite i 500 zeri piegherebbero il polinomio
    let max = 0;
    for (let i = 0; i < 1000; i++) max = Math.max(max, Math.abs(row.y[i]!));
    expect(max).toBeLessThan(0.5);
    for (let i = 1000; i < 1500; i++) expect(Number.isFinite(row.y[i]!)).toBe(true);
  });
});

describe('RowPacer', () => {
  const count = (rowSecs: number, navg: number, coreRows: number) => {
    const pacer = new RowPacer();
    let n = 0;
    for (let i = 0; i < coreRows; i++) if (pacer.add(Uint8Array.from([100, 120]), rowSecs, navg)) n++;
    return n;
  };
  it('WSJT-X pace: one row every 0.288 s, 52 rows in a 15 s period', () => {
    expect(count(0.288, 1, 225)).toBe(52);
    expect(count(0.288, 2, 225)).toBe(26);
  });
  it('JTDX pace: every core row, or one every N with N Avg', () => {
    expect(count(1 / 15, 1, 225)).toBe(225);
    expect(count(1 / 15, 3, 225)).toBe(75);
  });
  it('averages the amplitudes of the collected rows', () => {
    const pacer = new RowPacer();
    expect(pacer.add(Uint8Array.from([255]), 0.2, 1)).toBeNull();
    expect(pacer.add(Uint8Array.from([255]), 0.2, 1)).toBeNull();
    const out = pacer.add(Uint8Array.from([255]), 0.2, 1)!;
    expect(out[0]).toBeCloseTo(amplitudeOfByte(255), 5);
  });
});
