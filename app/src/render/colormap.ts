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

/** Colori dello spettro del motore: fondo blu, segnali gialli, i piu' forti bianchi. */
export const WATERFALL_STOPS: ColorStops = [
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

/**
 * Tavolozza di JTDX da 256 colori: lo stesso calcolo di `WFPalette::interpolate` (`WFPalette.cpp`). I punti sono distribuiti con
 * passo `256 / (punti - 1)` e tra due punti il colore sale in modo lineare; l'ultimo colore si raggiunge solo all'ultima voce.
 * Un solo punto parte dal nero, nessun punto va dal nero al bianco (`make_valid`).
 */
export function jtdxPaletteLut(points: ReadonlyArray<Rgb>): Uint8Array {
  let colours: Rgb[] = [...points];
  if (colours.length < 2) colours = [[0, 0, 0] as Rgb, ...colours];
  if (colours.length < 2) colours = [...colours, [255, 255, 255] as Rgb];
  const lut = new Uint8Array(LUT_SIZE * 4);
  const interval = LUT_SIZE / (colours.length - 1);
  for (let i = 0; i < LUT_SIZE; i++) {
    let prior = Math.floor(i / interval);
    if (prior >= colours.length - 1) prior--;
    const next = Math.min(prior + 1, colours.length - 1);
    const increment = i - interval * prior;
    const a = colours[prior]!;
    const b = colours[next]!;
    for (let ch = 0; ch < 3; ch++) {
      lut[i * 4 + ch] = Math.round(a[ch]! + (increment * (b[ch]! - a[ch]!)) / interval);
    }
    lut[i * 4 + 3] = 255;
  }
  return lut;
}
