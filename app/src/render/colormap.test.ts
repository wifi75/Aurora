import { describe, expect, it } from 'vitest';
import { WATERFALL_STOPS, NOTTE_RADIO_STOPS, buildLut, jtdxPaletteLut, lutColor } from './colormap';
import { JTDX_PALETTES, JTDX_PALETTE_NAMES } from './wfPalettes';

describe('colormap', () => {
  it('buildLut first and last stop match NOTTE_RADIO_STOPS', () => {
    const lut = buildLut(NOTTE_RADIO_STOPS);
    expect(lut.length).toBe(256 * 4);
    expect(Array.from(lut.slice(0, 4))).toEqual([8, 12, 24, 255]);
    expect(Array.from(lut.slice(-4))).toEqual([255, 250, 235, 255]);
  });

  it('buildLut passes near the intermediate stops', () => {
    const lut = buildLut(NOTTE_RADIO_STOPS);
    const [r, g, b] = lutColor(lut, 0.6);
    expect(Math.abs(r - 22)).toBeLessThanOrEqual(3);
    expect(Math.abs(g - 150)).toBeLessThanOrEqual(3);
    expect(Math.abs(b - 150)).toBeLessThanOrEqual(3);
  });

  it('lutColor clamps below 0 and above 1', () => {
    const lut = buildLut(NOTTE_RADIO_STOPS);
    expect(lutColor(lut, -5)).toEqual(lutColor(lut, 0));
    expect(lutColor(lut, 9)).toEqual(lutColor(lut, 1));
    expect(lutColor(lut, Number.NaN)).toEqual(lutColor(lut, 0));
  });

  it('the default the engine palette goes blue, then yellow, then white', () => {
    const lut = buildLut(WATERFALL_STOPS);
    const [r0, g0, b0] = lutColor(lut, 0.2);
    expect(b0).toBeGreaterThan(r0 + 40);
    expect(b0).toBeGreaterThan(g0);
    const [r1, g1, b1] = lutColor(lut, 0.82);
    expect(r1).toBeGreaterThan(200);
    expect(g1).toBeGreaterThan(200);
    expect(b1).toBeLessThan(80);
    expect(lutColor(lut, 1)).toEqual([255, 255, 255]);
  });

  it('rejects a palette with fewer than two stops', () => {
    expect(() => buildLut([[0, [0, 0, 0]]])).toThrow();
  });

  it('jtdxPaletteLut follows WFPalette::interpolate: 9 points, 32 entries per segment', () => {
    const lut = jtdxPaletteLut(JTDX_PALETTES.Default!);
    expect(lut.length).toBe(256 * 4);
    // le voci multiple di 32 sono i punti del file, in ordine
    JTDX_PALETTES.Default!.slice(0, 8).forEach((p, k) => expect(Array.from(lut.slice(k * 32 * 4, k * 32 * 4 + 3))).toEqual(p));
    // a meta' del primo tratto (nero -> 0,6,136) il colore e' a meta'
    expect(Array.from(lut.slice(16 * 4, 16 * 4 + 3))).toEqual([0, 3, 68]);
    // l'ultimo punto non si raggiunge: la voce 255 e' un passo prima del rosso finale
    expect(lut[255 * 4]).toBe(255);
    expect(lut[255 * 4 + 1]).toBeGreaterThan(51);
  });

  it('jtdxPaletteLut handles a single point and no point like make_valid', () => {
    expect(Array.from(jtdxPaletteLut([[200, 100, 50]]).slice(0, 3))).toEqual([0, 0, 0]);
    expect(Array.from(jtdxPaletteLut([]).slice(0, 3))).toEqual([0, 0, 0]);
    expect(Array.from(jtdxPaletteLut([]).slice(-4, -1))[0]).toBeGreaterThan(250);
  });

  it('bundles the 30 palettes of JTDX, Default first among the defaults', () => {
    expect(JTDX_PALETTE_NAMES).toHaveLength(30);
    expect(JTDX_PALETTE_NAMES).toContain('Default');
    for (const name of JTDX_PALETTE_NAMES) expect(JTDX_PALETTES[name]!.length).toBeGreaterThanOrEqual(9);
  });
});
