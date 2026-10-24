import { describe, expect, it } from 'vitest';
import { JTDX_STOPS, NOTTE_RADIO_STOPS, buildLut, lutColor } from './colormap';

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

  it('the default JTDX palette goes blue, then yellow, then white', () => {
    const lut = buildLut(JTDX_STOPS);
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
});
