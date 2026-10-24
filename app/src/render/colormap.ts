export type Rgb = readonly [number, number, number];
export type ColorStops = ReadonlyArray<readonly [number, Rgb]>;

/** Colormap della cascata per il tema "Notte radio": blu notte, verde acqua, ciano, giallo, bianco. */
export const NOTTE_RADIO_STOPS: ColorStops = [
  [0, [8, 12, 24]],
  [0.35, [12, 52, 78]],
  [0.6, [22, 150, 150]],
  [0.82, [63, 214, 200]],
  [0.93, [242, 200, 90]],
  [1, [255, 250, 235]],
];

/** Colori dello spettro di JTDX: fondo blu, segnali gialli, i piu' forti bianchi. */
export const JTDX_STOPS: ColorStops = [
  [0, [0, 0, 40]],
  [0.3, [0, 16, 140]],
  [0.5, [20, 60, 220]],
  [0.68, [70, 170, 235]],
  [0.82, [255, 230, 40]],
  [0.93, [255, 150, 20]],
  [1, [255, 255, 255]],
];

export const LUT_SIZE = 256;

/** Tabella RGBA (4 byte per voce) interpolata linearmente tra i punti. */
export function buildLut(stops: ColorStops, size = LUT_SIZE): Uint8Array {
  if (stops.length < 2) throw new Error('la colormap richiede almeno due punti');
  const lut = new Uint8Array(size * 4);
  for (let i = 0; i < size; i++) {
    const t = i / (size - 1);
    let seg = 1;
    while (seg < stops.length - 1 && t > stops[seg]![0]) seg++;
    const [t0, c0] = stops[seg - 1]!;
    const [t1, c1] = stops[seg]!;
    const k = t1 === t0 ? 0 : Math.min(1, Math.max(0, (t - t0) / (t1 - t0)));
    for (let ch = 0; ch < 3; ch++) lut[i * 4 + ch] = Math.round(c0[ch]! + (c1[ch]! - c0[ch]!) * k);
    lut[i * 4 + 3] = 255;
  }
  return lut;
}

/** Colore per un valore in [0, 1]; valori fuori intervallo o NaN vengono riportati nei limiti. */
export function lutColor(lut: Uint8Array, v: number): [number, number, number] {
  const size = lut.length / 4;
  const clamped = Number.isFinite(v) ? Math.min(1, Math.max(0, v)) : 0;
  const i = Math.round(clamped * (size - 1)) * 4;
  return [lut[i]!, lut[i + 1]!, lut[i + 2]!];
}
