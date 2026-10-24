import { describe, expect, it } from 'vitest';
import { NOTTE_RADIO_STOPS, buildLut, lutColor } from './colormap';

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

  it('rejects a palette with fewer than two stops', () => {
    expect(() => buildLut([[0, [0, 0, 0]]])).toThrow();
  });
});
